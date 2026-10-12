"""GEN-05.10 compact mechanical fixture: generate RAW OUT | validate RAW FIX.

The fixture is a mechanical projection of the genuine Paper outputs (expectations for the
column driver under the contract's replay rule) plus Vanilla cells as discriminating evidence.
Names are replaced by table positions only. Nothing is computed from a formula except digests and
the replay class, which is derived from the contract's policy table (route 1 here, route 2 in
validate from the projected tokens, route 3 in delivery/freeze.py).
"""
from pathlib import Path
import hashlib
import json
import sys

sys.dont_write_bytecode = True
MIN_Y, TOP_Y = -64, 319
INT_MIN = -2147483648
FILES = ('states.tsv', 'biomes.tsv', 'groups.tsv', 'chunks.tsv', 'columns.tsv', 'noises.tsv', 'vanilla.tsv', 'manifest.json')
SETTINGS = {'normal': 'minecraft:overworld', 'large_biomes': 'minecraft:large_biomes', 'amplified': 'minecraft:amplified',
            'single_biome_surface': 'minecraft:overworld'}
# Contract policy table (§9): resolver source P6 (MultiNoise(minecraft:overworld) presets), then the centre domain.
MULTINOISE_PRESETS = ('normal', 'large_biomes', 'amplified')
CENTRE_DOMAIN = (-1874999, 1874998)
P4 = {'y': (MIN_Y, TOP_Y), 'stone': (1, 4096)}
NOISES = ('badlands_surface', 'badlands_pillar', 'badlands_pillar_roof', 'iceberg_surface', 'iceberg_pillar', 'iceberg_pillar_roof')
SAMPLED_BIOMES = ('minecraft:eroded_badlands', 'minecraft:frozen_ocean', 'minecraft:deep_frozen_ocean')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data if isinstance(data, bytes) else data.encode()).hexdigest()


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


def tsv(header, rows):
    return '\t'.join(header) + '\n' + ''.join('\t'.join(str(c) for c in r) + '\n' for r in rows)


def unrle(text, length=384):
    out = []
    for run in text.split('.'):
        value, count = run.split('*')
        out.extend([value] * int(count))
    require(len(out) == length, 'RLE length')
    return out


def rle(values):
    out, i = [], 0
    while i < len(values):
        j = i
        while j < len(values) and values[j] == values[i]:
            j += 1
        out.append(f'{values[i]}*{j - i}')
        i = j
    return '.'.join(out)


def expand_calls(runs, outcomes):
    calls = []
    if runs != '-':
        for run in runs.split(';'):
            y, a, b, w, n = run.split(':')
            for k in range(int(n)):
                calls.append([int(y) - k, int(a) + k, int(b) - k, INT_MIN if w == '-' else int(w)])
    values = []
    if outcomes != '-':
        for run in outcomes.split('.'):
            value, count = run.split('*')
            values.extend([value] * int(count))
    require(len(values) == len(calls), 'Call outcome count')
    return [c + [v] for c, v in zip(calls, values)]


def writes_text(pairs):
    """Descending (y, state) pairs as runs 'ytop:count:state'."""
    out, i = [], 0
    while i < len(pairs):
        y, s = pairs[i]
        n = 1
        while i + n < len(pairs) and pairs[i + n] == (y - n, s):
            n += 1
        out.append(f'{y}:{n}:{s}')
        i += n
    return ';'.join(out) or '-'


def expand_writes(text):
    if text == '-':
        return []
    out = []
    for run in text.split(';'):
        y, n, s = run.split(':')
        out.extend((int(y) - k, s) for k in range(int(n)))
    return out


def policy_class(preset, cx, cz):
    if preset not in MULTINOISE_PRESETS:
        return 'refused:InconsistentResolver(Source)'
    if not (CENTRE_DOMAIN[0] <= cx <= CENTRE_DOMAIN[1] and CENTRE_DOMAIN[0] <= cz <= CENTRE_DOMAIN[1]):
        return 'refused:CentreOutsideDomain'
    return 'full'


def final_digest(columns_raw_ids):
    """SHA-256 of the comma-joined raw ids in FerrumC material order ((y+64)*16+lx)*16+lz."""
    values = []
    for y in range(384):
        for lx in range(16):
            for lz in range(16):
                values.append(columns_raw_ids[lx * 16 + lz][y])
    return sha(','.join(map(str, values)))


# ---------------------------------------------------------------- generate
def load(raw):
    data = {}
    for mode in ('paper', 'vanilla'):
        _, states = table(raw / mode / 'states.tsv')
        data[mode] = {
            'states': states,
            'groups': table(raw / mode / 'groups.tsv')[1],
            'chunks': {(c['group'], c['phase']): c for c in table(raw / mode / 'chunks.tsv')[1]},
            'columns': {(c['group'], int(c['lx']), int(c['lz'])): c for c in table(raw / mode / 'columns.tsv')[1]},
            'diags': {(d['group'], int(d['lx']), int(d['lz'])): d for d in table(raw / mode / 'diagnostics.tsv')[1]},
            'arrays': table(raw / mode / 'arrays.tsv')[1],
        }
    return data


def state_table(data):
    seen = {}
    for mode in ('paper', 'vanilla'):
        for s in data[mode]['states']:
            text, raw, _ = s['state'].split('|')
            flags = tuple(s[k] for k in ('is_air', 'has_fluid', 'is_stone_block', 'is_default_stone', 'is_water_block', 'is_lava_block', 'blocks_motion'))
            require(seen.setdefault(int(raw), (text, flags)) == (text, flags), 'State identity differs between modes')
    order = sorted(seen)
    return order, {raw: i for i, raw in enumerate(order)}, seen


def generate(raw, out):
    data = load(raw)
    order, index, seen = state_table(data)

    def remap(mode):
        return {int(s['index']): index[int(s['state'].split('|')[1])] for s in data[mode]['states']}
    pmap, vmap = remap('paper'), remap('vanilla')
    files = {}
    files['states.tsv'] = tsv(['index', 'state', 'raw_id', 'is_air', 'has_fluid', 'is_stone_block', 'is_default_stone',
                               'is_water_block', 'is_lava_block', 'blocks_motion'],
                              [[i, seen[r][0], r, *seen[r][1]] for i, r in enumerate(order)])
    biomes = sorted({c['biome_at_height_plus_1'] for m in ('paper', 'vanilla') for c in data[m]['columns'].values()})
    bindex = {b: i for i, b in enumerate(biomes)}
    files['biomes.tsv'] = tsv(['index', 'biome'], [[i, b] for i, b in enumerate(biomes)])
    digests = {}
    for a in data['paper']['arrays']:
        digests.setdefault(a['group'], []).append(sha(a['roster_indices_qy_qx_qz']))
    vgroups = {g['group']: g for g in data['vanilla']['groups']}
    group_rows, chunk_rows, column_rows, noise_rows, vanilla_rows = [], [], [], [], []
    counts = {'groups': 0, 'classes': {}, 'columns': 0, 'calls': 0, 'eroded_writes': 0, 'rule_writes': 0, 'frozen_writes': 0,
              'noise_rows': 0, 'vanilla_groups_with_final_difference': 0}
    for g in data['paper']['groups']:
        gid = g['group']
        cx, cz = int(g['cx']), int(g['cz'])
        cls = policy_class(g['preset'], cx, cz)
        counts['groups'] += 1
        counts['classes'][cls] = counts['classes'].get(cls, 0) + 1
        s0, fin = data['paper']['chunks'][(gid, 'S0')], data['paper']['chunks'][(gid, 'FINAL')]
        vfin = data['vanilla']['chunks'][(gid, 'FINAL')]
        s0c = [[pmap[int(v)] for v in unrle(t)] for t in s0['columns_rle'].split(';')]
        fc = [[pmap[int(v)] for v in unrle(t)] for t in fin['columns_rle'].split(';')]
        vfc = [[vmap[int(v)] for v in unrle(t)] for t in vfin['columns_rle'].split(';')]
        pdig = final_digest([[order[i] for i in col] for col in fc])
        vdig = final_digest([[order[i] for i in col] for col in vfc])
        group_rows.append([gid, g['preset'], SETTINGS[g['preset']], g['seed'], cx, cz, g['origin'], cls, g['preliminary_corners'],
                           g['sea_level'], g['noise_random_seed_lo_hi'], ','.join(digests[gid]), 1 if pdig == vdig else 0])
        chunk_rows.append([gid, s0['world_surface_wg'], s0['post_processing'], ';'.join(rle(c) for c in s0c),
                           fin['world_surface_wg'], fin['ocean_floor_wg'], fin['post_processing'], pdig])
        causes = []
        for k in range(256):
            lx, lz = divmod(k, 16)
            c = data['paper']['columns'][(gid, lx, lz)]
            after = [pmap[int(v)] for v in unrle(c['column_at_first_call_rle'])]
            eroded = [(y, after[y - MIN_Y]) for y in range(TOP_Y, MIN_Y - 1, -1) if after[y - MIN_Y] != s0c[k][y - MIN_Y]]
            calls = expand_calls(c['call_runs'], c['call_outcomes'])
            loop = list(after)
            outcomes = []
            for cl in calls:
                o = cl[4]
                if o == 'n':
                    outcomes.append('n')
                else:
                    outcomes.append(str(pmap[int(o)]))
                    loop[cl[0] - MIN_Y] = pmap[int(o)]
            frozen = [(y, fc[k][y - MIN_Y]) for y in range(TOP_Y, MIN_Y - 1, -1) if fc[k][y - MIN_Y] != loop[y - MIN_Y]]
            counts['columns'] += 1
            counts['calls'] += len(calls)
            counts['eroded_writes'] += len(eroded)
            counts['rule_writes'] += sum(1 for o in outcomes if o != 'n')
            counts['frozen_writes'] += len(frozen)
            column_rows.append([gid, lx, lz, bindex[c['biome_at_height_plus_1']], int(c['s0_world_surface_wg']) + 1,
                                writes_text(eroded), int(c['first_call_world_surface_wg']) + 1, c['surface_depth'], c['call_runs'],
                                rle(outcomes) if outcomes else '-', c['steep'], c['min_surface'], writes_text(frozen)])
            d, vd = data['paper']['diags'][(gid, lx, lz)], data['vanilla']['diags'][(gid, lx, lz)]
            if c['biome_at_height_plus_1'] in SAMPLED_BIOMES or lx == lz:
                counts['noise_rows'] += 1
                noise_rows.append([gid, lx, lz, ','.join(d[n].split('/')[1] for n in NOISES), ','.join(vd[n].split('/')[1] for n in NOISES),
                                   d['next_int_4'] + ',' + d['next_int_10'], d['next_double_bits'], vd['next_double_bits'], d['melt_slightly']])
            if fc[k] != vfc[k]:
                causes.append(str(k))
        if causes:
            counts['vanilla_groups_with_final_difference'] += 1
        vg = vgroups[gid]
        vanilla_rows.append([gid, vdig, vfin['world_surface_wg'] == fin['world_surface_wg'] and 1 or 0, ','.join(causes) or '-',
                             vg['preliminary_corners']])
    files['groups.tsv'] = tsv(['group', 'preset', 'settings', 'seed', 'cx', 'cz', 'origin', 'class', 'preliminary_corners', 'sea_level',
                               'noise_random_seed_lo_hi', 'paper_array_digests', 'vanilla_final_equal'], group_rows)
    files['chunks.tsv'] = tsv(['group', 's0_world_surface_wg', 's0_post_processing', 's0_columns_rle', 'final_world_surface_wg',
                               'final_ocean_floor_wg', 'final_post_processing', 'final_states_sha256'], chunk_rows)
    files['columns.tsv'] = tsv(['group', 'lx', 'lz', 'biome', 'height_plus_1', 'eroded_writes', 'first_call_height_plus_1', 'surface_depth',
                                'call_runs', 'call_outcomes', 'steep', 'min_surface', 'frozen_writes'], column_rows)
    files['noises.tsv'] = tsv(['group', 'lx', 'lz', 'paper_bits', 'vanilla_bits', 'paper_next_int_4_10', 'paper_next_double_bits',
                               'vanilla_next_double_bits', 'paper_melt'], noise_rows)
    files['vanilla.tsv'] = tsv(['group', 'vanilla_final_states_sha256', 'vanilla_final_world_surface_wg_equal', 'columns_differing_from_paper',
                                'vanilla_preliminary_corners'], vanilla_rows)
    manifest = {
        'scope': 'GEN-05.10 compact mechanical projection of genuine original SURFACE runs; Paper cells are the column-driver '
                 'expectations, Vanilla cells are discriminating evidence only',
        'raw_manifest_sha256': sha((raw / 'manifest.json').read_bytes()),
        'generator_sha256': sha(Path(__file__).read_bytes()),
        'counts': counts,
        'policy_table': {'resolver_source_presets': list(MULTINOISE_PRESETS), 'centre_domain': list(CENTRE_DOMAIN), 'p4_domains': P4,
                         'order': 'resolver source (P6) before centre domain'},
        'replay': {
            'full': 'Drive the whole chunk from the GEN-05.4 material (must equal s0_columns_rle), the GEN-05.5 arrays (digests), '
                    'the GEN-05.8 resolver and the GEN-05.3 corners; compare final_states_sha256, final_world_surface_wg, the '
                    'fluid marks as the set of final_post_processing positions, and the cfg(test) per-column trace (biome, '
                    'height_plus_1, eroded_writes, first_call_height_plus_1, surface_depth, every call input and outcome, steep '
                    'call/result/heights, min_surface, frozen_writes) exactly',
            'refused': 'Expect exactly the typed refusal named by the class, an Empty output buffer and no publication; never '
                       'compare original values of the group',
            'vanilla': 'Never an expectation; discriminating control for bit comparisons only'},
        'labels': {'differential': 'every cell is copied from genuine original output', 'derived': 'class (policy table), '
                   'eroded_writes/frozen_writes (genuine snapshot differences), digests'},
        'files': {},
    }
    return files, manifest


def write(out, files, manifest):
    out.mkdir(parents=True)
    for name, text in files.items():
        (out / name).write_text(text)
        manifest['files'][name] = sha(text)
    (out / 'manifest.json').write_text(json.dumps(manifest, sort_keys=True, indent=2) + '\n')


# ---------------------------------------------------------------- validate (route 2)
def validate(raw, fix):
    files, manifest = generate(raw, None)
    for name, text in files.items():
        manifest['files'][name] = sha(text)
    regenerated = all((fix / n).read_text() == t for n, t in files.items()) and \
        (fix / 'manifest.json').read_text() == json.dumps(manifest, sort_keys=True, indent=2) + '\n'
    require(regenerated, 'Fixture does not regenerate byte for byte')
    require(sorted(p.name for p in fix.iterdir()) == sorted(FILES), 'Fixture file set')
    report = check(raw, fix)
    report['regenerated_equal'] = regenerated
    return report


def check(raw, fix):
    """Route 2 on the projected tokens alone: expansion to raw, class derivation, reconstruction."""
    _, states = table(fix / 'states.tsv')
    flag = {k: [s[k] == '1' for s in states] for k in ('is_air', 'has_fluid', 'blocks_motion', 'is_water_block')}
    raw_id = [int(s['raw_id']) for s in states]
    name = [s['state'] for s in states]
    _, biomes = table(fix / 'biomes.tsv')
    data = load(raw)
    praw = {int(s['index']): s['state'].split('|')[0] for s in data['paper']['states']}
    vraw = {int(s['index']): s['state'].split('|')[0] for s in data['vanilla']['states']}
    _, groups = table(fix / 'groups.tsv')
    chunks = {c['group']: c for c in table(fix / 'chunks.tsv')[1]}
    cols = {}
    for c in table(fix / 'columns.tsv')[1]:
        cols.setdefault(c['group'], []).append(c)
    report = {'classes': {}, 'full_calls_checked_against_p4': 0, 'reconstructed_final_digests': 0, 'expanded_columns_equal_raw': 0,
              'noise_rows_equal_raw': 0, 'vanilla_digests_equal_raw': 0}
    for g in groups:
        gid = g['group']
        # Route 2: class from the projected tokens and the policy table only.
        if g['preset'] not in MULTINOISE_PRESETS:
            cls = 'refused:InconsistentResolver(Source)'
        elif not all(CENTRE_DOMAIN[0] <= int(g[k]) <= CENTRE_DOMAIN[1] for k in ('cx', 'cz')):
            cls = 'refused:CentreOutsideDomain'
        else:
            cls = 'full'
        require(cls == g['class'], 'Class differs on route 2: ' + gid)
        report['classes'][cls] = report['classes'].get(cls, 0) + 1
        ch = chunks[gid]
        s0 = [[int(v) for v in unrle(t)] for t in ch['s0_columns_rle'].split(';')]
        raw_s0 = [[praw[int(v)] for v in unrle(t)] for t in data['paper']['chunks'][(gid, 'S0')]['columns_rle'].split(';')]
        require([[name[v] for v in col] for col in s0] == raw_s0, 'S0 expansion differs from raw: ' + gid)
        final = [list(col) for col in s0]
        post = {}
        for part in ([] if ch['s0_post_processing'] == '-' else ch['s0_post_processing'].split(';')):
            sec, values = part.split(':')
            post[int(sec)] = values.split(',')
        rows = sorted(cols[gid], key=lambda c: (int(c['lx']), int(c['lz'])))
        require(len(rows) == 256, 'Column count')
        s0ws = [int(v) for v in ch['s0_world_surface_wg'].split(',')]
        for k, c in enumerate(rows):
            lx, lz = int(c['lx']), int(c['lz'])
            rc = data['paper']['columns'][(gid, lx, lz)]
            require(biomes[int(c['biome'])]['biome'] == rc['biome_at_height_plus_1'], 'Biome expansion')
            require(int(c['height_plus_1']) == s0ws[k] + 1 and int(rc['s0_world_surface_wg']) == s0ws[k], 'Height expansion')
            col = final[k]
            for y, s in expand_writes(c['eroded_writes']):
                require(flag['is_air'][col[y - MIN_Y]], 'Eroded write onto non-air')
                col[y - MIN_Y] = int(s)
            require([name[v] for v in col] == [praw[int(v)] for v in unrle(rc['column_at_first_call_rle'])], 'Post-prepass column: ' + gid)
            calls = expand_calls(c['call_runs'], c['call_outcomes'])
            raw_calls = expand_calls(rc['call_runs'], rc['call_outcomes'])
            require([cl[:4] for cl in calls] == [cl[:4] for cl in raw_calls], 'Call inputs expansion')
            require([name[int(cl[4])] if cl[4] != 'n' else 'n' for cl in calls] == [praw[int(cl[4])] if cl[4] != 'n' else 'n' for cl in raw_calls],
                    'Call outcome expansion')
            for cl in calls:
                if cls == 'full':
                    # Water: the recorded Integer.MIN_VALUE is None and every other value is an admitted Some.
                    require(P4['y'][0] <= cl[0] <= P4['y'][1] and P4['stone'][0] <= cl[1] <= P4['stone'][1]
                            and P4['stone'][0] <= cl[2] <= P4['stone'][1], 'Call outside a P4 domain in a full group: ' + gid)
                    report['full_calls_checked_against_p4'] += 1
                if cl[4] != 'n':
                    s = int(cl[4])
                    col[cl[0] - MIN_Y] = s
                    if flag['has_fluid'][s]:
                        post.setdefault((cl[0] - MIN_Y) >> 4, []).append(str(lx | ((cl[0] & 15) << 4) | (lz << 8)))
            for y, s in expand_writes(c['frozen_writes']):
                col[y - MIN_Y] = int(s)
            require(c['steep'] == rc['steep'] and c['min_surface'] == rc['min_surface'] and c['surface_depth'] == rc['surface_depth']
                    and int(c['first_call_height_plus_1']) == int(rc['first_call_world_surface_wg']) + 1, 'Scalar column cells')
            report['expanded_columns_equal_raw'] += 1
        rawfin = [[praw[int(v)] for v in unrle(t)] for t in data['paper']['chunks'][(gid, 'FINAL')]['columns_rle'].split(';')]
        require([[name[v] for v in col] for col in final] == rawfin, 'Reconstructed final chunk differs from raw: ' + gid)
        require(final_digest([[raw_id[v] for v in col] for col in final]) == ch['final_states_sha256'], 'Final digest')
        for key, pred in (('final_world_surface_wg', lambda s: not flag['is_air'][s]), ('final_ocean_floor_wg', lambda s: flag['blocks_motion'][s])):
            heights = []
            for col in final:
                top = MIN_Y - 1
                for y in range(TOP_Y, MIN_Y - 1, -1):
                    if pred(col[y - MIN_Y]):
                        top = y
                        break
                heights.append(str(top))
            require(','.join(heights) == ch[key], 'Recomputed ' + key)
        require((';'.join(f'{k}:' + ','.join(v) for k, v in sorted(post.items())) or '-') == ch['final_post_processing'], 'Post-processing order')
        report['reconstructed_final_digests'] += 1
        vrow = [v for v in table(fix / 'vanilla.tsv')[1] if v['group'] == gid][0]
        vfin = [[vraw[int(v)] for v in unrle(t)] for t in data['vanilla']['chunks'][(gid, 'FINAL')]['columns_rle'].split(';')]
        rid = {n: r for n, r in zip(name, raw_id)}
        require(final_digest([[rid[n] for n in col] for col in vfin]) == vrow['vanilla_final_states_sha256'], 'Vanilla digest')
        report['vanilla_digests_equal_raw'] += 1
    for n in table(fix / 'noises.tsv')[1]:
        d = data['paper']['diags'][(n['group'], int(n['lx']), int(n['lz']))]
        vd = data['vanilla']['diags'][(n['group'], int(n['lx']), int(n['lz']))]
        require(n['paper_bits'] == ','.join(d[k].split('/')[1] for k in NOISES) and n['vanilla_bits'] == ','.join(vd[k].split('/')[1] for k in NOISES)
                and n['paper_next_double_bits'] == d['next_double_bits'] and n['vanilla_next_double_bits'] == vd['next_double_bits']
                and n['paper_melt'] == d['melt_slightly'], 'Noise row expansion')
        report['noise_rows_equal_raw'] += 1
    report['python_optimize'] = sys.flags.optimize
    return report


def main():
    require(len(sys.argv) == 4 and sys.argv[1] in ('generate', 'validate'), 'generate RAW OUT | validate RAW FIX')
    raw, target = Path(sys.argv[2]).resolve(), Path(sys.argv[3]).resolve()
    if sys.argv[1] == 'generate':
        require(not target.exists(), 'Fresh fixture output required')
        files, manifest = generate(raw, target)
        write(target, files, manifest)
        print(json.dumps({'files': manifest['files'], 'counts': manifest['counts']}, sort_keys=True))
    else:
        print(json.dumps(validate(raw, target), sort_keys=True))


if __name__ == '__main__':
    main()
