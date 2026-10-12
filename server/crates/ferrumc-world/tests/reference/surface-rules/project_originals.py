"""Validate genuine original outputs and project them mechanically; no numerical expectation formula.

Projections only copy, count and compare original tokens. Coverage classification
of a recorded original noise value against its original threshold is a coverage
statistic, never a runtime expectation.
"""
from pathlib import Path
import collections
import hashlib
import json
import struct
import sys
sys.dont_write_bytecode = True

ROW_HEADER = ['tier', 'group', 'case', 'x', 'y', 'z', 'ctx_x', 'ctx_y', 'ctx_z', 'surface_depth', 'stone_above', 'stone_below',
              'water_height', 'update_xz', 'update_y', 'result', 'exception', 'computed', 'singletons', 'steep_heights',
              'min_surface', 'surface_secondary', 'biome_memo', 'column_surface_noise_and_draw', 'biome_direct',
              'cross_trees', 'paper_flat_bedrock']
MODES = ('vanilla', 'paper')
PAPER_SUBSTITUTION = ('2\t1\t2\tvertical_gradient\tminecraft:bedrock_floor;above_bottom:0;above_bottom:5',
                      '2\t1\t2\tpaper_optionally_flat_bedrock\tminecraft:bedrock_floor;above_bottom:0;above_bottom:5;is_roof=false')


class ProjectionFailure(ValueError):
    """Original output shape or invariant refused."""


def require(condition, message):
    if not condition:
        raise ProjectionFailure(message)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def tsv(path):
    lines = Path(path).read_text().splitlines()
    return lines[0].split('\t'), [line.split('\t') for line in lines[1:]]


def tree_lines(path):
    header, rows = tsv(path)
    require(header == ['id', 'parent', 'depth', 'kind', 'attributes', 'shared_with'], 'Tree header ' + str(path))
    return ['\t'.join(r[:5]) for r in rows], rows


def double_bits(hexbits):
    return struct.unpack('>d', int(hexbits, 16).to_bytes(8, 'big'))[0]


def threshold_bounds(tree_rows):
    bounds = {}
    for r in tree_rows:
        if r[3] == 'noise_threshold':
            key, low, high = r[4].split(';')
            bounds[int(r[0])] = (key, double_bits(low.split('/')[1]), double_bits(high.split('/')[1]))
    return bounds


def coverage(rows, tree_rows):
    kinds = {int(r[0]): r[3] for r in tree_rows}
    nodes = collections.defaultdict(collections.Counter)
    singles = collections.defaultdict(collections.Counter)
    sides = collections.defaultdict(collections.Counter)
    bounds = threshold_bounds(tree_rows)
    results = collections.Counter()
    tiers = collections.Counter()
    for r in rows:
        if r[0] not in ('G', 'C'):
            continue
        tiers[r[0]] += 1
        results[r[15].split('|')[0]] += 1
        if r[17] != '-':
            for entry in r[17].split(','):
                parts = entry.split(':')
                node = int(parts[0])
                nodes[node][parts[1]] += 1
                if node in bounds and len(parts) > 2:
                    value = double_bits(parts[2])
                    _, low, high = bounds[node]
                    sides[node]['below_min' if value < low else 'above_max' if value > high else 'within'] += 1
        if r[18] != '-':
            for entry in r[18].split(','):
                name, value = entry.split(':')
                singles[name][value] += 1
    lazy_kinds = {'biome', 'noise_threshold', 'vertical_gradient', 'paper_optionally_flat_bedrock', 'y_above', 'water', 'stone_depth'}
    lazy_nodes = sorted(n for n, k in kinds.items() if k in lazy_kinds)
    missing = {n: kinds[n] + ':' + ''.join(sorted(set('01') - set(nodes[n]))) for n in lazy_nodes if set(nodes[n]) != {'0', '1'}}
    one_sided = {n: dict(sides[n]) for n in bounds if not (sides[n]['below_min'] and sides[n]['within'] and (sides[n]['above_max'] or bounds[n][2] > 1e300))}
    return {'rows_by_tier': dict(tiers), 'lazy_condition_nodes': len(lazy_nodes),
            'lazy_nodes_with_both_outcomes': len(lazy_nodes) - len(missing), 'lazy_nodes_missing_outcome': missing,
            'singletons': {k: dict(v) for k, v in sorted(singles.items())},
            'noise_threshold_sides': {str(n): dict(sides[n]) for n in sorted(bounds)},
            'noise_thresholds_not_covering_every_side': {str(k): v for k, v in one_sided.items()},
            'results': dict(sorted(results.items()))}


def load_rows(path):
    header, rows = tsv(path)
    require(header == ROW_HEADER, 'Row header ' + str(path))
    for r in rows:
        if r[0] in ('G', 'C'):
            require(len(r) == len(ROW_HEADER), 'Row width')
    return rows


def compare_modes(vanilla, paper):
    def keyed(rows):
        out = {}
        for r in rows:
            if r[0] in ('G', 'C'):
                k = (r[0], r[1], r[2])
                require(k not in out, 'Duplicate original row key ' + str(k))
                out[k] = r
        return out
    v, p = keyed(vanilla), keyed(paper)
    fields = {'position': (3, 6), 'surface_depth': (9, 10), 'result': (15, 16), 'exception': (16, 17), 'computed': (17, 18),
              'singletons': (18, 19), 'steep_heights': (19, 20), 'min_surface': (20, 21), 'surface_secondary': (21, 22),
              'column_noise_and_draw': (23, 24), 'biome_direct': (24, 25)}
    diff = collections.Counter()
    examples = collections.defaultdict(list)
    noise_only = draw_only = 0
    for k in sorted(set(v) & set(p)):
        a, b = v[k], p[k]
        for name, (lo, hi) in fields.items():
            if a[lo:hi] != b[lo:hi]:
                diff[name] += 1
                if len(examples[name]) < 12:
                    examples[name].append({'key': '/'.join(k), 'vanilla': a[lo:hi], 'paper': b[lo:hi]})
        if a[23] != '-' and b[23] != '-':
            na, da = a[23].split(';')
            nb, db = b[23].split(';')
            noise_only += na != nb
            draw_only += da != db
    return {'common_rows': len(set(v) & set(p)), 'only_vanilla': len(set(v) - set(p)), 'only_paper': len(set(p) - set(v)),
            'only_vanilla_by_tier': dict(collections.Counter(k[0] for k in set(v) - set(p))),
            'only_paper_by_tier': dict(collections.Counter(k[0] for k in set(p) - set(v))),
            'field_differences': dict(diff), 'column_surface_noise_differs': noise_only, 'column_draw_differs': draw_only,
            'examples': dict(examples)}


def validate_and_project(output, pins):
    output = Path(output)
    header, centers = tsv(output / 'scan/centers.tsv')
    require(header == ['label', 'biome', 'kind', 'x', 'y', 'z', 'cx', 'cz', 'ring'], 'Genuine scan centre header')
    require(len({c[1] for c in centers}) == 35, 'Every declared target appears in the scan output')
    projection = output / 'projection'
    projection.mkdir()
    report = {'scan_centres': dict(collections.Counter(c[2] for c in centers))}
    trees, rows, counts, groups = {}, {}, {}, {}
    for mode in MODES:
        base = output / mode
        for name in ('rows.tsv', 'arrays.tsv', 'groups.tsv', 'counts.tsv', 'origins.tsv', 'tree-jar.tsv', 'tree-bootstrap.tsv', 'tree-registry.tsv', 'tree-variants.tsv'):
            require((base / name).is_file(), 'Missing original output ' + mode + '/' + name)
        jar, jar_rows = tree_lines(base / 'tree-jar.tsv')
        boot, _ = tree_lines(base / 'tree-bootstrap.tsv')
        reg, reg_rows = tree_lines(base / 'tree-registry.tsv')
        require(boot == reg and len(reg) == 428, 'Bootstrap and FerrumC-registry trees must be canonically equal')
        variants = tsv(base / 'tree-variants.tsv')[1]
        if mode == 'vanilla':
            require(jar == reg, 'Vanilla JAR tree must equal the registry tree')
            require(all(v[3] == 'true' for v in variants), 'Vanilla tree variants equal')
        else:
            changed = [(a, b) for a, b in zip(reg, jar) if a != b]
            require(changed == [PAPER_SUBSTITUTION], 'Paper JAR tree differs only by the bedrock-floor substitution')
            require(all(v[3] == ('true' if v[1] == 'jar' else 'false@2') for v in variants), 'Paper tree variant shape')
        trees[mode] = (jar, jar_rows, reg_rows)
        counts[mode] = dict(tsv_pairs(base / 'counts.tsv'))
        require(counts[mode]['cross_tree_mismatch'] == '0', 'Cross-tree outcome mismatch in ' + mode)
        rows[mode] = load_rows(base / 'rows.tsv')
        require(len(rows[mode]) <= 1000000, 'Declared per-mode row cap')
        bad = [r[:3] + r[15:17] for r in rows[mode] if r[0] in ('G', 'C') and r[16] != 'none']
        report[mode + '_rows_with_original_exception'] = len(bad)
        report[mode + '_exception_examples'] = bad[:10]
        groups[mode] = tsv(base / 'groups.tsv')[1]
        origins = dict(line.split('\t', 1) for line in (base / 'origins.tsv').read_text().splitlines())
        require(origins['helper/main'].endswith('$fresh-helper'), 'Helper origin')
        require(all(v.endswith('$original-inner') for k, v in origins.items() if k.startswith('original/') or k.startswith('actual/')), 'Original origins')
    require([g[:7] for g in groups['vanilla']] == [g[:7] for g in groups['paper']], 'Group identity across modes')
    arrays = {m: tsv(output / m / 'arrays.tsv')[1] for m in MODES}
    require([a[:3] for a in arrays['vanilla']] == [a[:3] for a in arrays['paper']], 'Stored biome array identity across originals')
    array_diff = [a[:3] for a, b in zip(arrays['vanilla'], arrays['paper']) if a[3] != b[3]]
    report['stored_biome_chunks_differing_vanilla_paper'] = len(array_diff)
    report['stored_biome_chunk_differences'] = array_diff
    report['stored_biome_values_differing'] = sum(x != y for a, b in zip(arrays['vanilla'], arrays['paper']) for x, y in zip(a[3].split(','), b[3].split(',')))
    report['genuine_surface_calls_equal_across_originals'] = counts['vanilla']['genuine_surface_calls'] == counts['paper']['genuine_surface_calls']
    (projection / 'tree.tsv').write_text('id\tparent\tdepth\tkind\tattributes\n' + ''.join(l + '\n' for l in trees['vanilla'][0]))
    (projection / 'paper-tree-substitution.json').write_text(json.dumps({'vanilla_and_registry': PAPER_SUBSTITUTION[0], 'paper_jar': PAPER_SUBSTITUTION[1]}, indent=2) + '\n')
    cov = {mode: coverage(rows[mode], trees[mode][1]) for mode in MODES}
    (projection / 'coverage.json').write_text(json.dumps(cov, sort_keys=True, indent=1) + '\n')
    modes = compare_modes(rows['vanilla'], rows['paper'])
    group_diff = {'preliminary_corners_differ': sum(a[7] != b[7] for a, b in zip(groups['vanilla'], groups['paper'])),
                  'final_heights_differ': sum(a[8] != b[8] for a, b in zip(groups['vanilla'], groups['paper'])),
                  'clay_bands_differ': sum(a[12] != b[12] for a, b in zip(groups['vanilla'], groups['paper'])),
                  'resolved_anchors_differ': sum(a[13] != b[13] for a, b in zip(groups['vanilla'], groups['paper']))}
    modes['groups'] = group_diff
    (projection / 'modes.json').write_text(json.dumps(modes, sort_keys=True, indent=1) + '\n')
    flat = collections.Counter()
    for r in rows['paper']:
        if r[0] == 'C' and r[26] != '-':
            flat['equal' if r[26] == r[15] else r[15].split('|')[0] + '->' + r[26].split('|')[0]] += 1
    report.update({'counts': counts, 'mode_comparison': {k: v for k, v in modes.items() if k != 'examples'},
                   'coverage_summary': {m: {k: cov[m][k] for k in ('rows_by_tier', 'lazy_condition_nodes', 'lazy_nodes_with_both_outcomes', 'lazy_nodes_missing_outcome', 'singletons', 'noise_thresholds_not_covering_every_side')} for m in MODES},
                   'paper_flat_bedrock_true_vs_default': dict(flat),
                   'diagnostics': {m: [r for r in rows[m] if r[0] == 'D'] for m in MODES}})
    return report


def tsv_pairs(path):
    return [line.split('\t') for line in Path(path).read_text().splitlines()]


if __name__ == '__main__':
    print(json.dumps(validate_and_project(Path(sys.argv[1]), None), sort_keys=True, indent=1))
