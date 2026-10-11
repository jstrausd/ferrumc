#!/usr/bin/env python3
"""Mechanical compact projection of the frozen GEN-05.8 original oracle.

`generate` copies genuine original outcomes into a fresh compact directory.
`validate` regenerates that projection, compares every byte with a committed
fixture and re-expands every committed row against the frozen source rows.

No selected-quart, vertical clamp, local mask, section or storage-index formula
supplies a value. Names are replaced by positions in explicit tables only; the
validator proves that every replacement expands back to the original token.
"""
import argparse
import collections
import csv
import hashlib
import json
from pathlib import Path
import sys
import tempfile

sys.dont_write_bytecode = True

FREEZE = '.codex-tmp/gen05_8/delivery/freeze-manifest.json'
FREEZE_SHA256 = 'd838de6c3af6d512d9b47741d8ed2be8b476337885bb0487d1a51435ea296cee'
PROJECTION = '.codex-tmp/gen05_8/final-normal/{mode}/projection/{name}.jsonl'
CASES = '.codex-tmp/gen05_8/design/cases.json'
ROSTER = '.codex-tmp/gen05_8/design/roster.tsv'
MODES = ('vanilla', 'paper')
TREES = ('final-normal', 'final-optimized')
NAMES = ('queries', 'arrays', 'histories', 'exceptions')
QUERY_COLUMNS = ['group', 'kind', 'seed', 'x', 'y', 'z', 'qx', 'qy', 'qz', 'cx', 'cz', 'outcome', 'resource']
ARRAY_COLUMNS = ['group', 'cx', 'cz', 'min_y', 'height', 'values']
HISTORY_COLUMNS = ['group', 'sequence', 'cx', 'cz', 'history', 'settings', 'source', 'min_y', 'height',
                   'initial_ordinal', 'initial_biome', 'final_ordinal', 'final_biome']
SOURCE_CLASSES = {
    'net.minecraft.world.level.biome.MultiNoiseBiomeSource': 'multinoise',
    'net.minecraft.world.level.biome.FixedBiomeSource': 'fixed',
    'net.minecraft.world.level.biome.TheEndBiomeSource': 'end',
}
SELECTION_RULE = ('Every original query, stored-value array and createBiomes history row of '
                  'final-normal in original order. Vanilla and Paper rows are stored once because '
                  'their complete query/array/history projections are byte-identical; each Rust row '
                  'is evaluated under both explicit zoom semantics. The 32 exception rows are '
                  'retained only as excluded-geometry evidence.')
PROVENANCE = ('Original Vanilla 1.21.8 obfuscated JAR and Paper 1.21.8 build 60 executed by the frozen '
              'GEN-05.8 helper. Vanilla ran with -Dmax.bg.threads=1, so createBiomes ran on one '
              'background worker and shared forward/reverse Vanilla histories are single-worker '
              'sequences, not production multi-worker scheduling. Paper ran createBiomes on the '
              'calling thread.')
POLICY = ('controlled_null_hook rows are FerrumC policy rows: the original LevelReader hook was '
          'observed outside the centre 3x3 window. Rust refuses them with OutsideNeighborhood at '
          'the delegated quart, and public block methods additionally apply the documented '
          'block X/Z +/-30000000 policy first. They are never original-equality rows.')


class FixtureFailure(ValueError):
    """Refuses incomplete, drifted or non-mechanical fixture input."""


def require(condition, message):
    if not condition:
        raise FixtureFailure(message)


def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()


def frozen_sources(repo):
    """Every consumed file is pinned by the frozen GEN-05.8 freeze manifest."""
    freeze = repo / FREEZE
    require(digest(freeze) == FREEZE_SHA256, 'GEN-05.8 freeze manifest drift')
    pins = json.loads(freeze.read_text())['authoritative_whole_files']
    consumed = {}
    for tree in TREES:
        for mode in MODES:
            for name in NAMES:
                consumed[PROJECTION.replace('final-normal', tree).format(mode=mode, name=name)] = None
    consumed[CASES] = None
    consumed[ROSTER] = None
    for name in consumed:
        require(name in pins, 'Unpinned fixture source ' + name)
        actual = digest(repo / name)
        require(actual == pins[name], 'Frozen source drift ' + name)
        consumed[name] = actual
    for name in ('queries', 'arrays', 'histories'):
        hashes = {consumed[PROJECTION.replace('final-normal', tree).format(mode=mode, name=name)]
                  for tree in TREES for mode in MODES}
        require(len(hashes) == 1, 'Vanilla/Paper or normal/-O projection differs: ' + name)
    for mode in MODES:
        require(consumed[PROJECTION.format(mode=mode, name='exceptions')] ==
                consumed[PROJECTION.replace('final-normal', 'final-optimized').format(mode=mode, name='exceptions')],
                'normal/-O exception evidence differs')
    return consumed


def rows(repo, name, mode='paper'):
    with (repo / PROJECTION.format(mode=mode, name=name)).open() as stream:
        return [json.loads(line) for line in stream]


def tables(repo):
    cases = json.loads((repo / CASES).read_text())
    roster = sorted((repo / ROSTER).read_text().split())
    require(len(roster) == len(set(roster)) == 65, 'Original 65-resource roster changed')
    groups = []
    for group in cases['groups']:
        entry = {'id': group['id'], 'kind': group['kind'], 'center': group['center']}
        if group['kind'] == 'axis':
            entry.update(min_y=group['min_y'], height=group['height'])
        else:
            entry.update(preset=group['preset'], dimension=group['dimension'], history=group['history'],
                         world_seed=str(group['world_seed']))
        groups.append(entry)
    return cases, roster, groups


def tsv(path, header, records):
    with path.open('x', newline='') as stream:
        writer = csv.writer(stream, delimiter='\t', lineterminator='\n')
        writer.writerow(header)
        for record in records:
            writer.writerow(record)


def generate(repo, out):
    require(not out.exists(), 'Fixture output must be fresh')
    sources = frozen_sources(repo)
    cases, roster, groups = tables(repo)
    group_index = {g['id']: i for i, g in enumerate(groups)}
    resource = {name: str(i) for i, name in enumerate(roster)}
    queries = rows(repo, 'queries')
    arrays = rows(repo, 'arrays')
    histories = rows(repo, 'histories')
    exceptions = rows(repo, 'exceptions')
    require(len(queries) == cases['counts']['queries'], 'Original query census')
    seeds = []
    seed_index = {}
    query_records = []
    census = collections.Counter()
    for r in queries:
        require(r['source_calls'] == r['chunk_calls'] == '1', 'Original source/chunk call count')
        require(r['manager_calls'] == ('1' if r['kind'] == 'G' else '0'), 'Original manager chronology')
        require(r['exception'] == 'none', 'Exception row inside query projection')
        require(r['outcome'] in ('value', 'controlled_null_hook'), 'Unexpected original outcome')
        require(r['fallback_calls'] == ('1' if r['outcome'] == 'controlled_null_hook' else '0'), 'Unlabelled hook')
        key = (r['seed_kind'], r['seed'], r['zoom_bits'])
        if key not in seed_index:
            seed_index[key] = len(seeds)
            seeds.append({'kind': r['seed_kind'], 'value': r['seed'], 'zoom_bits': r['zoom_bits']})
        value = r['outcome'] == 'value'
        require((r['resource'] in resource) == value and (value or r['resource'] == 'null'), 'Resource token')
        query_records.append([group_index[r['group']], r['kind'], seed_index[key], r['x'], r['y'], r['z'],
                              r['qx'], r['qy'], r['qz'], r['cx'], r['cz'], 'V' if value else 'H',
                              resource[r['resource']] if value else '-'])
        census[(r['group'].split('/')[0], r['kind'], r['outcome'])] += 1
    array_records = []
    for a in arrays:
        g = groups[group_index[a['group']]]
        if a['kind'] == 'axis':
            geometry = (a['geometry']['min_y'], a['geometry']['height'])
            require(a['generation'] is None, 'Axis array carries generation')
        else:
            geometry = (int(a['generation']['min_y']), int(a['generation']['height']))
        quarts = a['actual_quarts']
        require(len(a['values']) == len(quarts) == geometry[1] * 4, 'Incomplete original array')
        # Ordering evidence only: the original listing equals a lexicographic
        # (quart Y, quart X, quart Z) sort of its own actual quarts.
        require(quarts == sorted(quarts, key=lambda q: (q[1], q[0], q[2])), 'Original listing order changed')
        require(len({tuple(q) for q in quarts}) == len(quarts), 'Duplicate original quart')
        require({(q[0] >> 2, q[2] >> 2) for q in quarts} == {tuple(a['chunk'])}, 'Array quarts leave chunk')
        array_records.append([group_index[a['group']], a['chunk'][0], a['chunk'][1], geometry[0], geometry[1],
                              ','.join(resource[v] for v in a['values'])])
        require(g['kind'] == a['kind'], 'Array group kind')
    history_records = []
    for h in histories:
        source = SOURCE_CLASSES[h['source_class']]
        def biome(token):
            return '-' if token == 'none' else resource[token]
        history_records.append([group_index[h['group']], h['sequence'], h['cx'], h['cz'], h['history'],
                                h['settings'], source, h['min_y'], h['height'],
                                '-' if h['initial_ordinal'] == 'none' else h['initial_ordinal'],
                                biome(h['initial_biome']),
                                '-' if h['final_ordinal'] == 'none' else h['final_ordinal'],
                                biome(h['final_biome'])])
    out.mkdir(parents=True)
    tsv(out / 'queries.tsv', QUERY_COLUMNS, query_records)
    tsv(out / 'arrays.tsv', ARRAY_COLUMNS, array_records)
    tsv(out / 'histories.tsv', HISTORY_COLUMNS, history_records)
    manifest = {
        'scope': 'Compact mechanical projection of genuine original GEN-05.8 outcomes; no expected-value formula',
        'provenance': PROVENANCE,
        'policy_rows': POLICY,
        'selection_rule': SELECTION_RULE,
        'freeze_manifest_sha256': FREEZE_SHA256,
        'sources': sources,
        'roster': roster,
        'groups': groups,
        'seeds': seeds,
        'counts': {
            'queries': len(query_records),
            'arrays': len(array_records),
            'stored_values': sum(len(a['values']) for a in arrays),
            'histories': len(history_records),
            'exception_evidence_rows': len(exceptions),
            'query_census': {'/'.join(k): v for k, v in sorted(census.items())},
        },
        'excluded_exception_evidence': [{'id': e['id'], 'outcome': e['outcome']} for e in exceptions],
        'fixtures': {name: digest(out / name) for name in ('queries.tsv', 'arrays.tsv', 'histories.tsv')},
    }
    (out / 'manifest.json').write_text(json.dumps(manifest, indent=1, sort_keys=True) + '\n')
    return manifest


def expand_and_compare(repo, fixture):
    """Re-expands every committed row into the original tokens it replaced."""
    manifest = json.loads((fixture / 'manifest.json').read_text())
    roster, groups, seeds = manifest['roster'], manifest['groups'], manifest['seeds']

    def read(name):
        with (fixture / name).open(newline='') as stream:
            return list(csv.reader(stream, delimiter='\t'))

    committed = read('queries.tsv')
    require(committed[0] == QUERY_COLUMNS, 'Query header')
    original = rows(repo, 'queries')
    require(len(committed) - 1 == len(original), 'Query row count')
    for line, r in zip(committed[1:], original):
        g, kind, s, x, y, z, qx, qy, qz, cx, cz, outcome, res = line
        seed = seeds[int(s)]
        expanded = {'group': groups[int(g)]['id'], 'kind': kind, 'seed_kind': seed['kind'], 'seed': seed['value'],
                    'zoom_bits': seed['zoom_bits'], 'x': x, 'y': y, 'z': z, 'qx': qx, 'qy': qy, 'qz': qz,
                    'cx': cx, 'cz': cz,
                    'outcome': 'value' if outcome == 'V' else 'controlled_null_hook',
                    'resource': roster[int(res)] if outcome == 'V' else 'null'}
        require(all(r[k] == v for k, v in expanded.items()), 'Query row drift ' + r['id'])
    committed = read('arrays.tsv')
    original = rows(repo, 'arrays')
    require(committed[0] == ARRAY_COLUMNS and len(committed) - 1 == len(original), 'Array census')
    for line, a in zip(committed[1:], original):
        require(groups[int(line[0])]['id'] == a['group'] and [int(line[1]), int(line[2])] == a['chunk'], 'Array key')
        require([roster[int(v)] for v in line[5].split(',')] == a['values'], 'Array values drift ' + a['group'])
    committed = read('histories.tsv')
    original = rows(repo, 'histories')
    require(committed[0] == HISTORY_COLUMNS and len(committed) - 1 == len(original), 'History census')
    for line, h in zip(committed[1:], original):
        record = dict(zip(HISTORY_COLUMNS, line))
        require(groups[int(record['group'])]['id'] == h['group'], 'History group')
        for name in ('sequence', 'cx', 'cz', 'history', 'settings', 'min_y', 'height'):
            require(record[name] == h[name], 'History drift ' + name)
        require(SOURCE_CLASSES[h['source_class']] == record['source'], 'History source')
        for name in ('initial_ordinal', 'final_ordinal'):
            require(record[name] == ('-' if h[name] == 'none' else h[name]), 'History ordinal')
        for name in ('initial_biome', 'final_biome'):
            require(record[name] == ('-' if h[name] == 'none' else str(roster.index(h[name]))), 'History biome')
    for name, expected in manifest['fixtures'].items():
        require(digest(fixture / name) == expected, 'Fixture hash drift ' + name)


def validate(repo, fixture):
    frozen_sources(repo)
    with tempfile.TemporaryDirectory() as temporary:
        fresh = Path(temporary) / 'fixture'
        generate(repo, fresh)
        committed = sorted(p.name for p in fixture.iterdir())
        require(committed == sorted(p.name for p in fresh.iterdir()), 'Fixture membership')
        for name in committed:
            require((fixture / name).read_bytes() == (fresh / name).read_bytes(), 'Regenerated bytes differ ' + name)
    expand_and_compare(repo, fixture)
    frozen_sources(repo)
    return {name: digest(fixture / name) for name in sorted(p.name for p in fixture.iterdir())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=('generate', 'validate'))
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    args = parser.parse_args()
    repo = args.repo.resolve()
    if args.command == 'generate':
        manifest = generate(repo, args.fixture)
        print(json.dumps(manifest['counts'], sort_keys=True))
    else:
        print(json.dumps(validate(repo, args.fixture), sort_keys=True))


if __name__ == '__main__':
    main()
