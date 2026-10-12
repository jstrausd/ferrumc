"""Validate fresh original GEN-05.10 outputs and project them; second-route contract model checks.

Nothing here is an expectation source. The genuine outputs are the evidence. This module
(a) checks genuine invariants (U == R, post-NOISE equality, derivable heightmaps, untouched
neighbours and block entities), and (b) replays the contract's column-driver model (column loop,
eroded-badlands and frozen-ocean extensions, live WORLD_SURFACE_WG view, post-processing order)
from genuine recorded inputs and compares it with the genuine writes. (b) is labelled
"contract model check (derived)": it tests the contract text, it is not an oracle.
"""
from pathlib import Path
import math
import re
import struct
import sys

sys.dont_write_bytecode = True
MIN_Y, TOP_Y, HEIGHT = -64, 319, 384
INT_MIN = -2147483648
WAY_BELOW_MIN_Y = -2032 << 4
M64 = (1 << 64) - 1
# FerrumC centre domain: the complete 3x3 halo inside the admitted chunk domain [-1875000, 1874999].
CENTRE_DOMAIN = (-1874999, 1874998)
ADMITTED_PRESETS = ('normal', 'large_biomes', 'amplified')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def table(path):
    rows = path.read_text().split('\n')
    require(rows[-1] == '', 'Unterminated table ' + str(path))
    header = rows[0].split('\t')
    out = []
    for line in rows[1:-1]:
        cells = line.split('\t')
        require(len(cells) == len(header), 'Ragged row in ' + str(path))
        out.append(dict(zip(header, cells)))
    return header, out


def unrle(text, length=HEIGHT):
    out = []
    for run in text.split('.'):
        value, count = run.split('*')
        out.extend([int(value)] * int(count))
    require(len(out) == length, 'RLE length')
    return out


def expand_calls(runs, outcomes):
    calls = []
    if runs != '-':
        for run in runs.split(';'):
            y, a, b, w, n = run.split(':')
            y, a, b, n = int(y), int(a), int(b), int(n)
            w = INT_MIN if w == '-' else int(w)
            for k in range(n):
                calls.append([y - k, a + k, b - k, w])
    values = []
    if outcomes != '-':
        for run in outcomes.split('.'):
            value, count = run.split('*')
            values.extend([-1 if value == 'n' else int(value)] * int(count))
    require(len(values) == len(calls), 'Call outcome count')
    for c, v in zip(calls, values):
        c.append(v)
    return calls


# ---------------------------------------------------------------- original random (second route)
def s64(v):
    v &= M64
    return v - (1 << 64) if v >> 63 else v


def rotl(v, r):
    v &= M64
    return ((v << r) | (v >> (64 - r))) & M64


class Xoroshiro:
    def __init__(self, lo, hi):
        lo &= M64
        hi &= M64
        if lo == 0 and hi == 0:
            lo, hi = (-7046029254386353131) & M64, 7640891576956012809
        self.lo, self.hi = lo, hi

    def next_long(self):
        l, l1 = self.lo, self.hi
        l2 = (rotl((l + l1) & M64, 17) + l) & M64
        l1 ^= l
        self.lo = rotl(l, 49) ^ l1 ^ ((l1 << 21) & M64)
        self.hi = rotl(l1, 28)
        return l2

    def next_int_bound(self, bound):
        l = self.next_long() & 0xffffffff
        l1 = l * bound
        l2 = l1 & 0xffffffff
        if l2 < bound:
            i = ((-bound) & 0xffffffff) % bound
            while l2 < i:
                l = self.next_long() & 0xffffffff
                l1 = l * bound
                l2 = l1 & 0xffffffff
        return l1 >> 32

    def next_double(self, paper):
        bits = self.next_long() >> 11
        if paper:
            # Paper build 60 recompiled nextDouble as l2f / fmul 2^-53f / f2d.
            return struct.unpack('<f', struct.pack('<f', float(bits)))[0] * 2.0 ** -53
        return bits * 2.0 ** -53


def java_int_mul(a, b):
    v = (a * b) & 0xffffffff
    return v - (1 << 32) if v >> 31 else v


def get_seed(x, y, z):
    l = s64(java_int_mul(x, 3129871) ^ (z * 116129781) ^ y)
    l = s64(l * l * 42317861 + l * 11)
    return l >> 16


def at(seeds, x, y, z):
    lo, hi = seeds
    return Xoroshiro(get_seed(x, y, z) ^ lo, hi)


def java_floor(v):
    i = int(v)
    return i - 1 if v < i else i


def double_bits(text):
    return int(text.split('/')[1], 16)


def bits_double(bits):
    return struct.unpack('<d', struct.pack('<Q', bits))[0]


# ---------------------------------------------------------------- contract model (derived)
class States:
    def __init__(self, rows):
        self.rows = rows
        self.name = [r['state'].split('|')[0] for r in rows]
        self.air = [r['is_air'] == '1' for r in rows]
        self.fluid = [r['has_fluid'] == '1' for r in rows]
        self.stone_block = [r['is_stone_block'] == '1' for r in rows]
        self.default = [r['is_default_stone'] == '1' for r in rows]
        self.water = [r['is_water_block'] == '1' for r in rows]
        self.motion = [r['blocks_motion'] == '1' for r in rows]
        self.entity = [r['has_block_entity'] == '1' for r in rows]
        self.index = {n: i for i, n in enumerate(self.name)}

    VOID = 'void'

    def is_air(self, s):
        return s == self.VOID or self.air[s]

    def is_fluid(self, s):
        return s != self.VOID and self.fluid[s]


def get(column, y):
    return column[y - MIN_Y] if MIN_Y <= y <= TOP_Y else States.VOID


def top(states, column, predicate):
    for y in range(TOP_Y, MIN_Y - 1, -1):
        s = column[y - MIN_Y]
        if predicate(s):
            return y
    return MIN_Y - 1


def model_calls(states, column, i5):
    """SurfaceSystem.buildSurface column loop over the column after the eroded prepass."""
    calls, i6, i7, i8 = [], 0, INT_MIN, 2147483647
    for y in range(i5, MIN_Y - 1, -1):
        s = get(column, y)
        if states.is_air(s):
            i6, i7 = 0, INT_MIN
        elif states.is_fluid(s):
            if i7 == INT_MIN:
                i7 = y + 1
        else:
            if i8 >= y:
                i8 = WAY_BELOW_MIN_Y
                for y2 in range(y - 1, MIN_Y - 2, -1):
                    t = get(column, y2)
                    if states.is_air(t) or states.is_fluid(t):
                        i8 = y2 + 1
                        break
            i6 += 1
            if states.default[s]:
                calls.append([y, i6, y - i8 + 1, i7])
    return calls


def model_eroded(states, column, i4, d):
    surface, pillar, roof = (bits_double(double_bits(d[k])) for k in ('badlands_surface', 'badlands_pillar', 'badlands_pillar_roof'))
    mn = min(abs(surface * 8.25), pillar * 15.0)
    if mn <= 0.0:
        return [], 'min_le_0'
    ab = abs(roof * 1.5)
    floor = java_floor(64.0 + min(mn * mn * 2.5, math.ceil(ab * 50.0) + 24.0))
    if i4 > floor:
        return [], 'height_above_floor'
    branch = 'reached_min_y'
    for y in range(floor, MIN_Y - 1, -1):
        s = get(column, y)
        if s != States.VOID and states.stone_block[s]:
            branch = 'stone_stop'
            break
        if s != States.VOID and states.water[s]:
            return [], 'water_abort'
    writes, y = [], floor
    while y >= MIN_Y and states.is_air(get(column, y)):
        if y <= TOP_Y:
            writes.append(y)
        y -= 1
    return writes, branch + (':write' if writes else ':no_air')


def model_frozen(states, column, i4, min_surface, d, seeds, x, z, sea, paper):
    surface, pillar, roof = (bits_double(double_bits(d[k])) for k in ('iceberg_surface', 'iceberg_pillar', 'iceberg_pillar_roof'))
    mn = min(abs(surface * 8.25), pillar * 15.0)
    if mn <= 1.8:
        return [], 'min_le_1_8'
    ab = abs(roof * 1.5)
    min1 = min(mn * mn * 1.2, math.ceil(ab * 40.0) + 14.0)
    if d['melt_slightly'] == '1':
        min1 -= 2.0
    if min1 > 2.0:
        d3 = sea - min1 - 7.0
        min1 += sea
    else:
        min1, d3 = 0.0, 0.0
    d4 = min1
    rs = at(seeds, x, 0, z)
    i = 2 + rs.next_int_bound(4)
    i1 = sea + 18 + rs.next_int_bound(10)
    i2, writes = 0, []
    snow, ice = states.index.get('minecraft:snow_block'), states.index.get('minecraft:packed_ice')
    col = list(column)
    for mx in range(max(i4, int(min1) + 1), min_surface - 1, -1):
        s = get(col, mx)
        hit = (states.is_air(s) and mx < int(d4) and rs.next_double(paper) > 0.01) or \
              (s != States.VOID and states.water[s] and mx > int(d3) and mx < sea and d3 != 0.0 and rs.next_double(paper) > 0.15)
        if hit:
            if i2 <= i and mx > i1:
                state = ('snow', snow)
                i2 += 1
            else:
                state = ('ice', ice)
            if MIN_Y <= mx <= TOP_Y:
                writes.append((mx, state))
                col[mx - MIN_Y] = state[1]
    return writes, 'iceberg' if min1 > 0.0 else 'flat'


def pack(lx, y, lz):
    return lx | ((y & 15) << 4) | (lz << 8)


def parse_post(text):
    out = {}
    if text != '-':
        for part in text.split(';'):
            sec, values = part.split(':')
            out[int(sec)] = [int(v) for v in values.split(',')] if values else []
    return out


def post_text(lists):
    return ';'.join(f'{k}:' + ','.join(map(str, v)) for k, v in sorted(lists.items())) or '-'


# ---------------------------------------------------------------- per mode
def admitted(group):
    """Contract policy order: resolver source (P6), then the centre domain."""
    if group['preset'] not in ADMITTED_PRESETS:
        return False, 'InconsistentResolver(Source)'
    cx, cz = int(group['cx']), int(group['cz'])
    if not all(CENTRE_DOMAIN[0] <= v <= CENTRE_DOMAIN[1] for v in (cx, cz)):
        return False, 'CentreOutsideDomain'
    return True, '-'


def check_mode(run, mode):
    paper = mode == 'paper'
    _, srows = table(run / 'states.tsv')
    states = States(srows)
    require([int(r['index']) for r in srows] == list(range(len(srows))), 'State table order')
    require(not any(states.entity), 'No SURFACE-step state carries a block entity')
    _, groups = table(run / 'groups.tsv')
    _, chunks = table(run / 'chunks.tsv')
    _, columns = table(run / 'columns.tsv')
    _, diags = table(run / 'diagnostics.tsv')
    phase = {(c['group'], c['phase']): c for c in chunks}
    cols_by = {}
    for c in columns:
        cols_by.setdefault(c['group'], {})[int(c['lx']) * 16 + int(c['lz'])] = c
    diag_by = {}
    for d in diags:
        diag_by.setdefault(d['group'], {})[int(d['lx']) * 16 + int(d['lz'])] = d
    report = {'groups': len(groups), 'states': len(srows), 'model_mismatches': [], 'counts': {}}
    counts = report['counts']

    def bump(key, n=1):
        counts[key] = counts.get(key, 0) + n

    per_group = {}
    for g in groups:
        gid = g['group']
        require(g['generation'] == 'none', 'Original generation failed for ' + gid)
        require(g['u_exception'] == 'none' and g['r_exception'] == 'none', 'Original SURFACE exception in ' + gid)
        require(g['s0_equal_u_r'] == '1' and g['final_equal_u_r'] == '1', 'U/R generations or SURFACE results differ in ' + gid)
        require(g['neighbours_unchanged'] == '1' and [v for _, _, v in re.findall(r'(-?\d+),(-?\d+)=(-?\d+)', g['neighbour_highest_filled_section'])] == ['-1'] * 8,
                'Neighbour chunks touched in ' + gid)
        require(g['sea_level'] == '63' and g['min_gen_y_gen_depth'] == '-64,384', 'Generation context in ' + gid)
        require(g['orphan_min_surface'] == '-', 'Minimum surface computed by a column without a rule call in ' + gid)
        seeds = tuple(int(v) for v in g['noise_random_seed_lo_hi'].split(','))
        s0, fin = phase[(gid, 'S0')], phase[(gid, 'FINAL')]
        for p in (s0, fin):
            require(p['heightmaps'] == 'WORLD_SURFACE_WG,OCEAN_FLOOR_WG,n=2', 'Worldgen heightmaps only in ' + gid)
            require(p['block_entities'] == '0' and p['pending_block_entities'] == '0', 'Block entities at SURFACE in ' + gid)
        s0c = [unrle(t) for t in s0['columns_rle'].split(';')]
        fc = [unrle(t) for t in fin['columns_rle'].split(';')]
        for p, cs in ((s0, s0c), (fin, fc)):
            ws = [int(v) for v in p['world_surface_wg'].split(',')]
            of = [int(v) for v in p['ocean_floor_wg'].split(',')]
            for i in range(256):
                require(ws[i] == top(states, cs[i], lambda s: not states.air[s]), f'WORLD_SURFACE_WG not derivable {gid} {i}')
                require(of[i] == top(states, cs[i], lambda s: states.motion[s]), f'OCEAN_FLOOR_WG not derivable {gid} {i}')
        s0ws = [int(v) for v in s0['world_surface_wg'].split(',')]
        order = g['column_order_at_update_xz'].split(',')
        require(len(order) == 256 and all(o == f'{k}@{k}' for k, o in enumerate(order)), 'Column order x outer, z inner, one updateXZ each: ' + gid)
        post = parse_post(s0['post_processing'])
        live = list(s0ws)
        cx, cz = int(g['cx']), int(g['cz'])
        mismatch = []
        gc = cols_by[gid]
        per_group[gid] = {'columns': {}}
        for k in range(256):
            lx, lz = divmod(k, 16)
            c, d = gc[k], diag_by[gid][k]
            x, z = cx * 16 + lx, cz * 16 + lz
            require(int(c['x']) == x and int(c['z']) == z and int(c['s0_world_surface_wg']) == s0ws[k], 'Column coordinates')
            require(c['biome_at_height_plus_1'] == d['biome'], 'Column biome in diagnostics')
            biome = c['biome_at_height_plus_1']
            i4 = s0ws[k] + 1
            after = unrle(c['column_at_first_call_rle'])
            # Eroded-badlands prepass: genuine difference S0 -> first call, against the model.
            eroded = [y for y in range(MIN_Y, TOP_Y + 1) if after[y - MIN_Y] != s0c[k][y - MIN_Y]]
            require(all(states.air[s0c[k][y - MIN_Y]] and states.default[after[y - MIN_Y]] for y in eroded), 'Eroded writes are air -> default block')
            if biome == 'minecraft:eroded_badlands':
                want, branch = model_eroded(states, s0c[k], i4, d)
                bump('eroded_branch:' + branch)
                if sorted(want) != eroded:
                    mismatch.append(('eroded', gid, k))
            else:
                require(not eroded, 'Eroded writes outside eroded_badlands ' + gid)
            if eroded:
                bump('eroded_columns_with_writes')
                bump('eroded_writes', len(eroded))
            i5 = int(c['first_call_world_surface_wg']) + 1
            require(i5 - 1 == top(states, after, lambda s: not states.air[s]), 'First-call height equals the post-prepass column top')
            live[k] = i5 - 1
            calls = expand_calls(c['call_runs'], c['call_outcomes'])
            require(len(calls) == int(c['calls']), 'Call count')
            if [cl[:4] for cl in calls] != model_calls(states, after, i5):
                mismatch.append(('calls', gid, k))
            bump('calls', len(calls))
            if len({cl[0] - cl[2] + 1 for cl in calls}) >= 2:
                bump('multi_run_columns')
            for cl in calls:
                if cl[3] != INT_MIN and MIN_Y <= cl[3] - 1 <= TOP_Y and states.name[after[cl[3] - 1 - MIN_Y]].startswith('minecraft:lava'):
                    bump('calls_with_lava_water_height')
            # Rule writes in loop order; live heights for the steep view and post-processing marks.
            loop = list(after)
            steep_call = int(c['steep'].split(':')[0]) if c['steep'] != '-' else -1
            for n, cl in enumerate(calls):
                if n == steep_call:
                    hs = [int(v) for v in c['steep'].split(':')[2].split(',')]
                    view = [live[lx * 16 + max(lz - 1, 0)], live[lx * 16 + min(lz + 1, 15)], live[max(lx - 1, 0) * 16 + lz], live[min(lx + 1, 15) * 16 + lz]]
                    if hs != view:
                        mismatch.append(('steep_view', gid, k))
                    pre = [s0ws[lx * 16 + max(lz - 1, 0)], s0ws[lx * 16 + min(lz + 1, 15)], s0ws[max(lx - 1, 0) * 16 + lz], s0ws[min(lx + 1, 15) * 16 + lz]]
                    bump('steep_tests')
                    if hs != pre:
                        bump('steep_tests_reading_changed_heights')
                if cl[4] >= 0:
                    y = cl[0]
                    loop[y - MIN_Y] = cl[4]
                    bump('rule_writes')
                    if states.fluid[cl[4]]:
                        post.setdefault((y - MIN_Y) >> 4, []).append(pack(lx, y, lz))
                        bump('rule_fluid_writes')
                    if states.air[cl[4]]:
                        bump('rule_air_writes')
                    live[k] = top(states, loop, lambda s: not states.air[s])
            frozen = [y for y in range(MIN_Y, TOP_Y + 1) if fc[k][y - MIN_Y] != loop[y - MIN_Y]]
            if biome in ('minecraft:frozen_ocean', 'minecraft:deep_frozen_ocean'):
                require(c['min_surface'] != '-', 'Frozen-ocean column without its minimum surface')
                want, branch = model_frozen(states, loop, i4, int(c['min_surface']), d, seeds, x, z, 63, paper)
                bump('frozen_branch:' + branch + ':melt' + d['melt_slightly'])
                got = [(y, fc[k][y - MIN_Y]) for y in frozen]
                if [(y, s[1]) for y, s in want] != sorted(got, reverse=True):
                    mismatch.append(('frozen', gid, k))
                for y in frozen:
                    bump('frozen_writes')
                    if states.water[loop[y - MIN_Y]]:
                        bump('frozen_writes_on_water')
                    if states.name[fc[k][y - MIN_Y]] == 'minecraft:snow_block':
                        bump('frozen_snow_writes')
            else:
                require(not frozen, 'Frozen writes outside frozen oceans ' + gid)
            live[k] = top(states, fc[k], lambda s: not states.air[s])
            # Draws: the genuine fresh noiseRandom.at(x,0,z) against the second-route Xoroshiro.
            rs = at(seeds, x, 0, z)
            got_draws = [rs.next_int_bound(4), rs.next_int_bound(10)] + [struct.unpack('<Q', struct.pack('<d', rs.next_double(paper)))[0] for _ in range(4)]
            want_draws = [int(d['next_int_4']), int(d['next_int_10'])] + [int(v, 16) for v in d['next_double_bits'].split(',')]
            require(got_draws == want_draws, f'Second-route Xoroshiro differs from genuine draws {gid} {k}')
            bump('columns')
            per_group[gid]['columns'][k] = biome
        require(post_text(post) == fin['post_processing'], 'Post-processing = post-NOISE marks + fluid rule writes in write order: ' + gid)
        for values in parse_post(fin['post_processing']).values():
            bump('post_processing_marks', len(values))
            bump('post_processing_duplicates', len(values) - len(set(values)))
        report['model_mismatches'].extend(mismatch)
        ok, kind = admitted(g)
        per_group[gid]['class'] = 'full' if ok else 'refused:' + kind
    classes = {}
    for gid, info in per_group.items():
        classes[info['class']] = classes.get(info['class'], 0) + 1
    report['group_classes_under_contract_policy'] = classes
    report['model_mismatch_count'] = len(report['model_mismatches'])
    report['model_mismatches'] = report['model_mismatches'][:50]
    return report, per_group, states


def validate_and_project(output, pins=None):
    report = {'scope': 'Genuine invariants checked; contract model check (derived) compared with genuine writes'}
    modes = {}
    for mode in ('vanilla', 'paper'):
        run = output / mode
        if (run / 'groups.tsv').is_file():
            report[mode], groups, states = check_mode(run, mode)
            modes[mode] = (groups, states)
    if len(modes) == 2:
        report['vanilla_vs_paper'] = compare_modes(output)
    return report


def first_cause(vn, pn, vs, ps, a, b):
    """First differing stage of one column, in buildSurface order (derived classification)."""
    if vs != ps:
        return 'post_noise_terrain'
    if a['biome_at_height_plus_1'] != b['biome_at_height_plus_1']:
        return 'column_biome'
    if [vn[i].split('|')[0] for i in unrle(a['column_at_first_call_rle'])] != [pn[i].split('|')[0] for i in unrle(b['column_at_first_call_rle'])]:
        return 'eroded_prepass'
    ca, cb = expand_calls(a['call_runs'], a['call_outcomes']), expand_calls(b['call_runs'], b['call_outcomes'])
    if a['surface_depth'] != b['surface_depth'] or [c[:4] for c in ca] != [c[:4] for c in cb]:
        return 'call_inputs'
    if [vn[c[4]].split('|')[0] if c[4] >= 0 else 'none' for c in ca] != [pn[c[4]].split('|')[0] if c[4] >= 0 else 'none' for c in cb]:
        return 'rule_outcomes'
    return 'frozen_postpass'


def compare_modes(output):
    out = {}
    tables = {}
    for mode in ('vanilla', 'paper'):
        _, srows = table(output / mode / 'states.tsv')
        names = [r['state'] for r in srows]
        _, chunks = table(output / mode / 'chunks.tsv')
        _, columns = table(output / mode / 'columns.tsv')
        _, diags = table(output / mode / 'diagnostics.tsv')
        tables[mode] = (names, {(c['group'], c['phase']): c for c in chunks},
                        {(c['group'], c['lx'], c['lz']): c for c in columns}, {(d['group'], d['lx'], d['lz']): d for d in diags})
    vn, vc, vcol, vd = tables['vanilla']
    pn, pc, pcol, pd = tables['paper']
    require(set(vc) == set(pc), 'Same group/phase set in both modes')
    groups = sorted({g for g, _ in pc})
    diff = {'groups': len(groups), 's0_columns_differ': 0, 'final_columns_differ': 0, 'final_columns_differ_with_equal_s0': 0,
            'column_biome_differs': 0, 'surface_depth_differs': 0, 'call_outcomes_differ': 0, 'groups_with_final_difference': [],
            'noise_bits_differ': {k: 0 for k in ('badlands_surface', 'badlands_pillar', 'badlands_pillar_roof', 'iceberg_surface', 'iceberg_pillar', 'iceberg_pillar_roof')},
            'next_int_differs': 0, 'next_double_bits_differ': 0, 'melt_differs': 0, 'columns': 0,
            'final_difference_first_cause': {k: 0 for k in ('post_noise_terrain', 'column_biome', 'eroded_prepass', 'call_inputs',
                                                             'rule_outcomes', 'frozen_postpass')}}
    for gid in groups:
        vs = [[vn[i].split('|')[0] for i in unrle(t)] for t in vc[(gid, 'S0')]['columns_rle'].split(';')]
        ps = [[pn[i].split('|')[0] for i in unrle(t)] for t in pc[(gid, 'S0')]['columns_rle'].split(';')]
        vf = [[vn[i].split('|')[0] for i in unrle(t)] for t in vc[(gid, 'FINAL')]['columns_rle'].split(';')]
        pf = [[pn[i].split('|')[0] for i in unrle(t)] for t in pc[(gid, 'FINAL')]['columns_rle'].split(';')]
        any_final = False
        for k in range(256):
            diff['columns'] += 1
            key = (gid, str(k // 16), str(k % 16))
            if vs[k] != ps[k]:
                diff['s0_columns_differ'] += 1
            a, b = vcol[key], pcol[key]
            if vf[k] != pf[k]:
                diff['final_columns_differ'] += 1
                any_final = True
                if vs[k] == ps[k]:
                    diff['final_columns_differ_with_equal_s0'] += 1
                diff['final_difference_first_cause'][first_cause(vn, pn, vs[k], ps[k], a, b)] += 1
            if a['biome_at_height_plus_1'] != b['biome_at_height_plus_1']:
                diff['column_biome_differs'] += 1
            if a['surface_depth'] != b['surface_depth']:
                diff['surface_depth_differs'] += 1
            if [vn[i].split('|')[0] if i >= 0 else 'none' for i in [c[4] for c in expand_calls(a['call_runs'], a['call_outcomes'])]] != \
               [pn[i].split('|')[0] if i >= 0 else 'none' for i in [c[4] for c in expand_calls(b['call_runs'], b['call_outcomes'])]]:
                diff['call_outcomes_differ'] += 1
            da, db = vd[key], pd[key]
            for n in diff['noise_bits_differ']:
                if da[n] != db[n]:
                    diff['noise_bits_differ'][n] += 1
            if (da['next_int_4'], da['next_int_10']) != (db['next_int_4'], db['next_int_10']):
                diff['next_int_differs'] += 1
            if da['next_double_bits'] != db['next_double_bits']:
                diff['next_double_bits_differ'] += 1
            if da['melt_slightly'] != db['melt_slightly']:
                diff['melt_differs'] += 1
        if any_final:
            diff['groups_with_final_difference'].append(gid)
    out.update(diff)
    return out


if __name__ == '__main__':
    import json
    print(json.dumps(validate_and_project(Path(sys.argv[1]).resolve()), sort_keys=True, indent=1))
