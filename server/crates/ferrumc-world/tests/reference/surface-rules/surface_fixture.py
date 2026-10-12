"""Compact mechanical fixture projection of genuine GEN-05.9 original outcomes, plus validator (round 3).

generate RAW OUT  -- RAW is a completed oracle output directory, OUT is fresh.
validate RAW FIX  -- regenerates into memory, compares bytes, then independently expands
                     every fixture row back to the raw Paper/Vanilla row tokens and
                     re-derives the replay table along a second route.

The projection copies original tokens and replaces names with table positions.
Selection only chooses which genuine rows are kept; no expectation is computed.

Round 2 (Root decisions after review FINDINGS B1):
- The raw `paper_flat_bedrock` cells were evaluated on a context that never received
  `updateXZ` (review B1). They are invalid and not authoritative and are no longer
  projected. The round-1 row selection is kept unchanged so that every other byte stays
  stable; the flat cell acts there only as a row picker for genuine default-mode rows.
- A1: `replay.tsv` declares, per row, its column-run start and whether the complete run
  prefix is in the fixture; `steep-heights.tsv` carries the steep-probe heights view.
- A2: `tree-paper.tsv` and `paper-tree-substitution.json` carry the Paper tree form.
- H8: exception, cross-tree and biome-memo cells are checked explicitly by `validate`.

Round 3 (Root decisions after review round 2, R2-B1): rows outside the P4 admission domains
(y, stone depth above/below, water) get the third replay class `refused` with the expected typed
refusal kind; they demand that refusal without a partial result, never original equality. `full`
now requires that the whole run prefix is in the fixture *and* admitted, because the original filled
per-column caches with refused inputs. `validate` re-derives all three classes and the refusal kind
along a second route from the projected rows. The raw oracle outputs, the row selection and every
fixture file except `replay.tsv` and `manifest.json` are unchanged.
"""
from pathlib import Path
import collections
import hashlib
import json
import sys
sys.dont_write_bytecode = True

SAMPLE_BYTE = 8          # keep rows whose selection hash starts with a byte < 8 (1/32)
COVERAGE_PER_OUTCOME = 32
INVALID_FLAT = ('ungültig, nicht maßgeblich (fabrizierter Kontext, Review B1): raw paper/rows.tsv column '
                'paper_flat_bedrock, paper/counts.tsv paper_flat_compared/paper_flat_differ and manifest report '
                'paper_flat_bedrock_true_vs_default were evaluated on a generateFlatBedrock=true context that never '
                'received updateXZ; they are not original outcomes for the stated position and are not projected here')
PER_BLOCK_KINDS = ('biome', 'vertical_gradient', 'paper_optionally_flat_bedrock', 'y_above', 'water', 'stone_depth')
# P4 admission domains of the contract (FerrumC policy) and the evaluate refusal order after the active
# column check: y, stone depth above, stone depth below, water. Water None is the original i32::MIN.
I32_MIN = -2147483648
Y_DOMAIN = (-64, 319)
STONE_DEPTH_DOMAIN = (1, 4096)
REFUSAL_ORDER = ('BlockOutsideGenerationHeight (y outside [-64, 319])',
                 'InvalidStoneDepth (stone depth above outside 1..=4096, then below)',
                 'InvalidWaterHeight (Some(i32::MIN); unreachable from the fixture, whose water None is the original i32::MIN)')
REPLAY_RULE = {
    'run': 'raw rows from one original updateXZ to the next on the recording context: starts at the raw row whose '
           'column_surface_noise_and_draw cell is set (fixture column != -), continues over consecutive raw ordinals '
           'with equal tier, group, update_xz, ctx_x and ctx_z',
    'full': 'every raw ordinal from run_start to this row is a fixture row and none of them is refused (the whole '
            'run prefix is admitted). Replay: begin_column at run_start, evaluate the run rows in ordinal order; '
            'result, surface depth, complete computed trace with sampler bits, singletons, steep heights, min '
            'surface, secondary and column cell must equal (original equality)',
    'partial': 'admitted row whose run prefix is incomplete or contains a refused row (the original filled per-column '
               'caches there with inputs the runtime refuses). Replay this row alone after begin_column; only result, '
               'surface depth, biome and '
               'the computed entries whose tree kind is per block (' + ', '.join(PER_BLOCK_KINDS) + ') plus the '
               'temperature singleton must equal. Per-column entries (noise_threshold with noise bits, steep, hole, '
               'above, steep heights, min surface, secondary, column cell) are evidence only, because an earlier '
               'call outside the fixture may have filled those caches in the original',
    'refused': 'row outside the P4 admission domains. Replay this row alone after begin_column; evaluate must return '
               'exactly the refusal kind in column refusal (first failing check in the order y, stone depth above, '
               'stone depth below, water), with no partial result and no state change: work_used, the active column '
               'and every per-column cache stay as before the call. No original value of the row is an expectation; '
               'all of them stay evidence of the original outside the policy domains',
    'heights': 'final = groups.tsv final_world_surface_wg (centre chunk after genuine buildSurface; controlled rows '
               'never write); steep-probe = steep-heights.tsv for that gid (synthetic stone chunk of the steep probe, '
               'copied from the y of the raw steep/lx,lz/top rows). validate re-checks every recorded steep_heights '
               'cell against the declared view',
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest_bytes(data):
    return hashlib.sha256(data).hexdigest()


def read_rows(path):
    lines = Path(path).read_text().splitlines()
    return lines[0].split('\t'), [l.split('\t') for l in lines[1:]]


def tables(raw):
    header, paper = read_rows(raw / 'paper/rows.tsv')
    _, vanilla = read_rows(raw / 'vanilla/rows.tsv')
    groups = [l.split('\t') for l in (raw / 'paper/groups.tsv').read_text().splitlines()[1:]]
    vgroups = [l.split('\t') for l in (raw / 'vanilla/groups.tsv').read_text().splitlines()[1:]]
    arrays = [l.split('\t') for l in (raw / 'paper/arrays.tsv').read_text().splitlines()[1:]]
    varrays = [l.split('\t') for l in (raw / 'vanilla/arrays.tsv').read_text().splitlines()[1:]]
    tree = (raw / 'projection/tree.tsv').read_text()
    return header, paper, vanilla, groups, vgroups, arrays, varrays, tree


def select(paper, vanilla, flat_picker=True):
    """Declared selection over raw Paper row ordinals (unchanged from round 1).

    flat_picker=False only counts which rows the invalid flat cell alone keeps (review R2-H3)."""
    keep = set()
    vkey = {(r[0], r[1], r[2]): r for r in vanilla if r[0] in ('G', 'C')}
    seen = collections.Counter()
    for i, r in enumerate(paper):
        if r[0] == 'D':
            keep.add(i)
            continue
        label = r[2]
        if r[17] != '-':
            for entry in r[17].split(','):
                node, outcome = entry.split(':')[:2]
                if seen[(node, outcome)] < COVERAGE_PER_OUTCOME:
                    seen[(node, outcome)] += 1
                    keep.add(i)
        if r[18] != '-':
            for entry in r[18].split(','):
                if seen[entry] < COVERAGE_PER_OUTCOME:
                    seen[entry] += 1
                    keep.add(i)
        v = vkey.get((r[0], r[1], r[2]))
        if v is None or v[15] != r[15] or v[9] != r[9] or v[24] != r[24]:
            keep.add(i)
        # Row picker only (invalid flat cell, review B1); the flat cell itself is not projected.
        if flat_picker and r[26] not in ('-', r[15]):
            keep.add(i)
        if label.startswith(('end/', 'steep/', 'extreme/', 'holecolumn/')) or r[1].startswith('hole/') and r[0] == 'G':
            keep.add(i)
        if hashlib.sha256('|'.join(r[:3]).encode()).digest()[0] < SAMPLE_BYTE:
            keep.add(i)
    return sorted(keep), vkey


def run_starts(paper):
    """Route 1: a run starts where the recording context first observed a new updateXZ (column cell set)."""
    starts, current = {}, None
    for i, r in enumerate(paper):
        if r[0] == 'D':
            current = None
            continue
        if r[23] != '-':
            current = i
        require(current is not None, 'row without column-run start ' + str(i))
        s = paper[current]
        require((s[0], s[1], s[13], s[6], s[8]) == (r[0], r[1], r[13], r[6], r[8]), 'column run identity ' + str(i))
        starts[i] = current
    return starts


def p4_refusal(y, stone_above, stone_below, water):
    """Expected typed P4 refusal kind of one block input, or '-' when admitted."""
    if not Y_DOMAIN[0] <= y <= Y_DOMAIN[1]:
        return 'BlockOutsideGenerationHeight'
    if not STONE_DEPTH_DOMAIN[0] <= stone_above <= STONE_DEPTH_DOMAIN[1]:
        return 'InvalidStoneDepth'
    if not STONE_DEPTH_DOMAIN[0] <= stone_below <= STONE_DEPTH_DOMAIN[1]:
        return 'InvalidStoneDepth'
    if water is not None and water == I32_MIN:
        return 'InvalidWaterHeight'
    return '-'


def raw_refusal(r):
    """Route 1: refusal kind from the raw Paper row (ctx_y, stone_above, stone_below, water_height)."""
    water = int(r[12])
    return p4_refusal(int(r[7]), int(r[10]), int(r[11]), None if water == I32_MIN else water)


def steep_views(paper, gids):
    """Steep-probe heights view per group: the y of every raw steep/lx,lz/top row (the placed stone top)."""
    views = {}
    for r in paper:
        if r[0] == 'C' and r[2].startswith('steep/') and r[2].endswith('/top'):
            lx, lz = (int(v) for v in r[2].split('/')[1].split(','))
            view = views.setdefault(gids[r[1]], {})
            require((lx, lz) not in view and int(r[4]) == int(r[7]), 'steep probe row identity')
            view[(lx, lz)] = r[4]
    for view in views.values():
        require(sorted(view) == [(lx, lz) for lx in range(16) for lz in range(16)], 'complete steep probe view')
    return views


def generate(raw):
    header, paper, vanilla, groups, vgroups, arrays, varrays, tree = tables(raw)
    keep, vkey = select(paper, vanilla)
    flat_only = len(set(keep) - set(select(paper, vanilla, flat_picker=False)[0]))
    gids = {g[0]: i for i, g in enumerate(groups)}
    kinds = {l.split('\t')[0]: l.split('\t')[3] for l in tree.splitlines()[1:]}
    states, biomes, patterns, bands = {}, {}, {}, {}
    def idx(table, key):
        if key not in table:
            table[key] = len(table)
        return table[key]
    columns = {}
    out_rows = ['ordinal\tgid\ttier\tx\ty\tz\tsurface_depth\tstone_above\tstone_below\twater\tresult\tbiome\tpattern\tsteep_heights\tmin_surface\tsecondary\tcolumn\tnoise_values\tgradient_draws\tvanilla']
    for i in keep:
        r = paper[i]
        if r[0] == 'D':
            out_rows.append('\t'.join([str(i), '-', 'D', *(['-'] * 16), '|'.join(r[1:])]))
            continue
        gid = gids[r[1]]
        computed, noise_values, draws = [], [], []
        if r[17] != '-':
            for entry in r[17].split(','):
                parts = entry.split(':')
                computed.append(parts[0] + ':' + parts[1])
                if len(parts) > 2:
                    require(kinds[parts[0]] in ('noise_threshold', 'vertical_gradient'), 'sampler value on unexpected node kind')
                    (noise_values if kinds[parts[0]] == 'noise_threshold' else draws).append(parts[0] + '=' + parts[2])
        pattern = idx(patterns, ','.join(computed) + '|' + r[18])
        column = '-'
        if r[23] != '-':
            column = idx(columns, (gid, r[6], r[8], r[23]))
        v = vkey.get((r[0], r[1], r[2]))
        vanilla_cell = '-' if v is None else ('=' if v[15] == r[15] and v[9] == r[9] and v[17] == r[17] and v[23] == r[23] and v[24] == r[24] and v[21] == r[21] else
                                              '|'.join([str(idx(states, v[15])), v[9], v[17], v[23], str(idx(biomes, v[24])), v[21]]))
        out_rows.append('\t'.join([str(i), str(gid), r[0], r[6], r[7], r[8], r[9], r[10], r[11], '-' if r[12] == '-2147483648' else r[12],
                                   str(idx(states, r[15])), str(idx(biomes, r[24])), str(pattern), r[19], r[20], r[21], str(column),
                                   ','.join(noise_values) or '-', ','.join(draws) or '-', vanilla_cell]))
    group_lines = ['gid\tgroup\tpreset\tseed\tcx\tcz\trecording\torigin\tpreliminary_corners\tsea_level\tmin_gen_y\tgen_depth\tclay_bands\tresolved_anchors\tfinal_world_surface_wg\tvanilla_final_world_surface_wg']
    for i, g in enumerate(groups):
        v = vgroups[i]
        require(v[:7] == g[:7], 'group identity')
        band = ','.join(str(idx(bands, b)) for b in g[12].split(','))
        group_lines.append('\t'.join([str(i), *g[:8], g[9], g[10], g[11], band, g[13], g[8], '=' if v[8] == g[8] else v[8]]))
    array_lines = ['gid\tcx\tcz\tpaper_sha256\tvanilla_sha256']
    for a, v in zip(arrays, varrays):
        require(a[:3] == v[:3], 'array identity')
        pa, va = digest_bytes(a[3].encode()), digest_bytes(v[3].encode())
        array_lines.append('\t'.join([str(gids[a[0]]), a[1], a[2], pa, '=' if pa == va else va]))
    column_lines = ['column\tgid\tx\tz\tsurface_noise_and_draw']
    for (gid, x, z, text), i in sorted(columns.items(), key=lambda e: e[1]):
        column_lines.append('\t'.join([str(i), str(gid), x, z, text]))
    # A1: column-run replay classes and the declared heights view per row.
    starts, kept = run_starts(paper), set(keep)
    replay_lines = ['ordinal\trun_start\treplay\theights\trefusal']
    replay_counts, refusal_counts = collections.Counter(), collections.Counter()
    for i in keep:
        r = paper[i]
        if r[0] == 'D':
            continue
        s = starts[i]
        refusal = raw_refusal(r)
        if refusal != '-':
            replay = 'refused'
            refusal_counts[r[0] + '/' + refusal + '/' + r[2].split('/')[0]] += 1
        elif all(j in kept and raw_refusal(paper[j]) == '-' for j in range(s, i + 1)):
            replay = 'full'
        else:
            replay = 'partial'
            if all(j in kept for j in range(s, i + 1)):
                replay_counts[r[0] + '/partial_complete_prefix_with_refused_row'] += 1
        heights = 'steep-probe' if r[2].startswith('steep/') else 'final'
        replay_counts[r[0] + '/' + replay] += 1
        replay_lines.append('\t'.join([str(i), str(s), replay, heights, refusal]))
    views = steep_views(paper, gids)
    steep_lines = ['gid\tsteep_probe_world_surface_wg']
    for gid in sorted(views):
        steep_lines.append(str(gid) + '\t' + ','.join(views[gid][(lx, lz)] for lx in range(16) for lz in range(16)))
    # A2: Paper tree form, projected exactly like projection/tree.tsv (first five canonical columns).
    paper_tree_lines = (raw / 'paper/tree-jar.tsv').read_text().splitlines()
    require(paper_tree_lines[0].split('\t') == ['id', 'parent', 'depth', 'kind', 'attributes', 'shared_with'], 'Paper tree header')
    paper_tree = 'id\tparent\tdepth\tkind\tattributes\n' + ''.join('\t'.join(l.split('\t')[:5]) + '\n' for l in paper_tree_lines[1:])
    substitution = (raw / 'projection/paper-tree-substitution.json').read_text()
    sub = json.loads(substitution)
    changed = [(a, b) for a, b in zip(tree.splitlines(), paper_tree.splitlines()) if a != b]
    require(len(tree.splitlines()) == len(paper_tree.splitlines()) and changed == [(sub['vanilla_and_registry'], sub['paper_jar'])],
            'Paper tree differs from the registry form exactly by the declared substitution')
    files = {
        'tree.tsv': tree,
        'tree-paper.tsv': paper_tree,
        'paper-tree-substitution.json': substitution,
        'groups.tsv': '\n'.join(group_lines) + '\n',
        'arrays.tsv': '\n'.join(array_lines) + '\n',
        'columns.tsv': '\n'.join(column_lines) + '\n',
        'rows.tsv': '\n'.join(out_rows) + '\n',
        'replay.tsv': '\n'.join(replay_lines) + '\n',
        'steep-heights.tsv': '\n'.join(steep_lines) + '\n',
        'states.tsv': 'index\tstate\n' + ''.join(f'{i}\t{s}\n' for s, i in sorted(states.items(), key=lambda e: e[1])),
        'biomes.tsv': 'index\tbiome\n' + ''.join(f'{i}\t{s}\n' for s, i in sorted(biomes.items(), key=lambda e: e[1])),
        'patterns.tsv': 'index\tcomputed_and_singletons\n' + ''.join(f'{i}\t{s}\n' for s, i in sorted(patterns.items(), key=lambda e: e[1])),
        'bands.tsv': 'index\tstate\n' + ''.join(f'{i}\t{s}\n' for s, i in sorted(bands.items(), key=lambda e: e[1])),
    }
    freeze = raw / 'manifest.json'
    manifest = {'scope': 'Compact mechanical projection of genuine GEN-05.9 original outcomes (round 3); Paper rows are the production '
                         'expectations for PaperFlatBedrock::Disabled only, Vanilla cells are discriminating evidence only; '
                         'no expectation formula; no flat-bedrock expectation exists; rows outside the P4 policy domains '
                         'expect the typed refusal only (replay class refused)',
                'round': 3,
                'invalid_evidence': {'paper_flat_bedrock': INVALID_FLAT},
                'source_manifest_sha256': digest_bytes(freeze.read_bytes()),
                'raw_sha256': {n: digest_bytes((raw / n).read_bytes()) for n in (
                    'paper/rows.tsv', 'vanilla/rows.tsv', 'paper/groups.tsv', 'vanilla/groups.tsv', 'paper/arrays.tsv', 'vanilla/arrays.tsv',
                    'projection/tree.tsv', 'paper/tree-jar.tsv', 'projection/paper-tree-substitution.json')},
                'selection': {'coverage_per_node_outcome': COVERAGE_PER_OUTCOME, 'sample_hash_first_byte_below': SAMPLE_BYTE,
                              'always': 'diagnostic rows; Vanilla result/surface-depth/biome differences; end/steep/extreme/holecolumn '
                                        'controlled rows; genuine hole-column G rows; rows whose invalid round-1 flat cell differs '
                                        '(row picker only, kept for byte stability; their flat cell is not projected)'},
                'replay': REPLAY_RULE,
                'p4_refusal_order': list(REFUSAL_ORDER),
                'paper_tree': 'tree-paper.tsv is the Paper JAR tree (Paper data form); tree.tsv is the Vanilla JAR = bootstrap = '
                              'FerrumC registry form; they differ exactly by paper-tree-substitution.json (node 2)',
                'validate_invariants': 'every kept G/C row: raw exception none, cross_trees "=" (C) or "-" (G, not cross-evaluated), '
                                       'biome_memo "-" or equal to biome_direct; '
                                       'every recorded Paper steep_heights cell equals the declared heights view',
                'counts': {'raw_paper_rows': len(paper), 'kept_rows': len(keep), 'groups': len(groups), 'columns': len(columns),
                           'states': len(states), 'biomes': len(biomes), 'patterns': len(patterns),
                           'replay': dict(sorted(replay_counts.items())), 'refused_by_tier_kind_case': dict(sorted(refusal_counts.items())),
                           'kept_only_by_invalid_flat_picker': flat_only, 'steep_probe_groups': len(views)},
                'files': {n: digest_bytes(t.encode()) for n, t in sorted(files.items())}}
    files['manifest.json'] = json.dumps(manifest, sort_keys=True, indent=2, ensure_ascii=False) + '\n'
    return files


def expand_and_compare(raw, fix):
    """Independent expansion: rebuild raw tokens from fixture tables only, compare with raw rows."""
    _, paper = read_rows(raw / 'paper/rows.tsv')
    _, vanilla = read_rows(raw / 'vanilla/rows.tsv')
    vkey = {(r[0], r[1], r[2]): r for r in vanilla if r[0] in ('G', 'C')}
    load = lambda name: [l.split('\t') for l in (fix / name).read_text().splitlines()[1:]]
    states = {r[0]: r[1] for r in load('states.tsv')}
    biomes = {r[0]: r[1] for r in load('biomes.tsv')}
    patterns = {r[0]: r[1] for r in load('patterns.tsv')}
    groups = {r[0]: r[1] for r in load('groups.tsv')}
    columns = {r[0]: r for r in load('columns.tsv')}
    checked = 0
    invariants = collections.Counter()
    for f in load('rows.tsv'):
        r = paper[int(f[0])]
        if f[2] == 'D':
            require('|'.join(r[1:]) == f[19], 'diagnostic expansion')
            checked += 1
            continue
        computed, singles = patterns[f[12]].split('|')
        bits = dict(e.split('=') for e in (f[17].split(',') if f[17] != '-' else []) + (f[18].split(',') if f[18] != '-' else []))
        trace = ','.join(e + (':' + bits[e.split(':')[0]] if e.split(':')[0] in bits else '') for e in computed.split(',')) if computed else '-'
        col = columns[f[16]] if f[16] != '-' else None
        expected = {1: groups[f[1]], 6: f[3], 7: f[4], 8: f[5], 9: f[6], 10: f[7], 11: f[8], 12: '-2147483648' if f[9] == '-' else f[9],
                    15: states[f[10]], 17: trace, 18: singles, 19: f[13], 20: f[14], 21: f[15], 23: '-' if col is None else col[4], 24: biomes[f[11]]}
        require(r[0] == f[2], 'tier expansion')
        for k, value in expected.items():
            require(r[k] == value, f'row {f[0]} field {k}: {r[k]!r} != {value!r}')
        if col is not None:
            require(groups[col[1]] == r[1] and col[2] == r[6] and col[3] == r[8], 'column identity')
        # H8: cells the fixture does not carry are pinned explicitly.
        require(r[16] == 'none', f'row {f[0]}: original exception {r[16]!r}')
        # Only controlled rows are cross-evaluated on the bootstrap and registry trees.
        require(r[25] == ('=' if r[0] == 'C' else '-'), f'row {f[0]}: cross-tree cell {r[25]!r}')
        require(r[22] in ('-', r[24]), f'row {f[0]}: biome memo {r[22]!r} differs from direct {r[24]!r}')
        invariants['exception_none'] += 1
        invariants['cross_trees_equal_C' if r[0] == 'C' else 'cross_trees_not_evaluated_G'] += 1
        invariants['biome_memo_set' if r[22] != '-' else 'biome_memo_unset'] += 1
        v = vkey.get((r[0], r[1], r[2]))
        if f[19] == '=':
            require(v is not None and (v[15], v[9], v[17], v[23], v[24], v[21]) == (r[15], r[9], r[17], r[23], r[24], r[21]), 'vanilla equality expansion')
        elif f[19] != '-':
            res, sd, comp, colv, bio, sec = f[19].split('|')
            require(v is not None and (v[15], v[9], v[17], v[23], v[24], v[21]) == (states[res], sd, comp, colv, biomes[bio], sec), 'vanilla difference expansion')
        checked += 1
    return checked, dict(sorted(invariants.items()))


def replay_route2(raw, fix):
    """Route 2 for replay.tsv: runs as maximal blocks of consecutive raw rows with equal tier, group and update_xz."""
    _, paper = read_rows(raw / 'paper/rows.tsv')
    starts, block_start = {}, None
    for i, r in enumerate(paper):
        if r[0] == 'D':
            block_start = None
            continue
        prev = paper[i - 1] if i > 0 else None
        if block_start is None or prev is None or (prev[0], prev[1], prev[13]) != (r[0], r[1], r[13]):
            block_start = i
            require(r[23] != '-', 'run block must start with the column cell ' + str(i))
        starts[i] = block_start
    load = lambda name: [l.split('\t') for l in (fix / name).read_text().splitlines()[1:]]
    fixture_rows = {int(f[0]): f for f in load('rows.tsv')}
    kept = set(fixture_rows)

    def refusal2(f):
        """Route 2: refusal kind from the projected fixture tokens (y, stone_above, stone_below, water '-' = None)."""
        y, above, below = int(f[4]), int(f[7]), int(f[8])
        if y < -64 or y > 319:
            return 'BlockOutsideGenerationHeight'
        if above < 1 or above > 4096 or below < 1 or below > 4096:
            return 'InvalidStoneDepth'
        if f[9] != '-' and int(f[9]) == -2147483648:
            return 'InvalidWaterHeight'
        return '-'
    refused2 = {i: refusal2(f) for i, f in fixture_rows.items() if f[2] != 'D'}
    classes = collections.Counter()
    groups = {g[0]: g for g in load('groups.tsv')}
    gid_of = {g[1]: g[0] for g in groups.values()}
    views = {s[0]: s[1].split(',') for s in load('steep-heights.tsv')}
    seen = 0
    for e in load('replay.tsv'):
        i, s = int(e[0]), int(e[1])
        r = paper[i]
        require(starts[i] == s, 'replay run start ' + e[0])
        if refused2[i] != '-':
            expected = 'refused'
        elif set(range(s, i + 1)) <= kept and all(refused2[j] == '-' for j in range(s, i + 1)):
            expected = 'full'
        else:
            expected = 'partial'
        require(e[2] == expected, 'replay class ' + e[0])
        require(e[4] == refused2[i], 'replay refusal kind ' + e[0])
        classes[r[0] + '/' + expected] += 1
        require(e[3] == ('steep-probe' if r[2].startswith('steep/') else 'final'), 'replay heights ' + e[0])
        seen += 1
    require(seen == sum(1 for i in kept if paper[i][0] != 'D'), 'replay table covers every kept G/C row')
    # Every recorded original steep read must equal the declared heights view (input consistency, not an expectation).
    steep = collections.Counter()
    for r in paper:
        if r[0] == 'D' or r[19] == '-':
            continue
        gid = gid_of[r[1]]
        view = views[gid] if r[2].startswith('steep/') else groups[gid][14].split(',')
        lx, lz = int(r[6]) & 15, int(r[8]) & 15
        h = lambda a, b: view[a * 16 + b]
        read = ','.join([h(lx, max(lz - 1, 0)), h(lx, min(lz + 1, 15)), h(max(lx - 1, 0), lz), h(min(lx + 1, 15), lz)])
        require(read == r[19], f'steep heights view mismatch at raw row {r[:3]}')
        steep[r[0] + ('/steep-probe' if r[2].startswith('steep/') else '/final')] += 1
    return seen, dict(sorted(classes.items())), dict(sorted(steep.items()))


def main():
    require(len(sys.argv) == 4 and sys.argv[1] in ('generate', 'validate'), 'generate RAW OUT | validate RAW FIX')
    raw, target = Path(sys.argv[2]).resolve(), Path(sys.argv[3]).resolve()
    files = generate(raw)
    if sys.argv[1] == 'generate':
        require(not target.exists(), 'fresh fixture output required')
        target.mkdir()
        for name, text in files.items():
            (target / name).write_bytes(text.encode())
        print(json.dumps({'files': {n: len(t.encode()) for n, t in files.items()}, 'manifest_sha256': digest_bytes(files['manifest.json'].encode())}, sort_keys=True))
        return
    for name, text in files.items():
        require((target / name).read_bytes() == text.encode(), 'regenerated fixture differs: ' + name)
    require(sorted(p.name for p in target.iterdir()) == sorted(files), 'fixture membership')
    expanded, invariants = expand_and_compare(raw, target)
    replay_rows, classes, steep_reads = replay_route2(raw, target)
    print(json.dumps({'regenerated_equal': True, 'expanded_rows_equal_raw': expanded, 'invariants_h8': invariants,
                      'replay_rows_rederived': replay_rows, 'replay_classes_rederived': classes,
                      'steep_heights_view_consistent': steep_reads}, sort_keys=True))


if __name__ == '__main__':
    main()
