"""GEN-05.9 runtime A5: mechanical projection of genuine clay_bands_offset values.

generate EXT RAW OUT  -- EXT is a completed extended-helper run (vanilla/ and paper/ clay.tsv),
                         RAW the frozen round-2 raw output (paper/rows.tsv, paper/groups.tsv); OUT is fresh.
validate EXT RAW FIX  -- regenerates in memory, compares bytes, expands every fixture row back to the
                         raw clay tokens and re-derives the column set along a second route.

Column set (review R3-H2, mechanical): the distinct (group, ctx_x, ctx_z) of the Paper G/C rows of
RAW/paper/rows.tsv whose biome_direct is minecraft:badlands, minecraft:eroded_badlands or
minecraft:wooded_badlands. Pass criterion of the runtime test: in every listed column the runtime Paper
basis `clay_bands_offset.sample(x, 0, z)` has exactly the genuine Paper f64 bits, and the Vanilla basis
has exactly the genuine Vanilla bits (control); any deviation means A5 is not met. The projection only
copies original tokens and replaces group names by fixture gid; no value is computed.
"""
from pathlib import Path
import hashlib
import json
import sys
sys.dont_write_bytecode = True

BADLANDS = ('minecraft:badlands', 'minecraft:eroded_badlands', 'minecraft:wooded_badlands')
CRITERION = ('every listed column: runtime Paper-basis clay_bands_offset.sample(x, 0, z) f64 bits == paper bits '
             '(genuine Paper SurfaceSystem.clayBandsOffsetNoise.getValue) and runtime Vanilla-basis bits == vanilla '
             'bits (genuine Vanilla value, control); any deviation means A5 is not met')
COLUMN_SET = ('distinct (group, ctx_x, ctx_z) of the Paper G/C rows of the frozen round-2 raw paper/rows.tsv whose '
              'biome_direct is one of ' + ', '.join(BADLANDS))


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def clay_values(path):
    """Genuine clay token per (group, x, z); every repeated observation must carry the same token."""
    lines = Path(path).read_text().splitlines()
    require(lines[0] == 'group\tx\tz\tclay_bands_offset', 'clay header ' + str(path))
    values = {}
    for line in lines[1:]:
        group, x, z, token = line.split('\t')
        require(values.setdefault((group, x, z), token) == token, 'inconsistent genuine clay value ' + line)
    return values


def column_set(raw):
    """Route 1: raw Paper rows, columns ctx_x (6), ctx_z (8), biome_direct (24)."""
    columns = []
    seen = set()
    for line in (raw / 'paper/rows.tsv').read_text().splitlines()[1:]:
        r = line.split('\t')
        if r[0] in ('G', 'C') and r[24] in BADLANDS and (r[1], r[6], r[8]) not in seen:
            seen.add((r[1], r[6], r[8]))
            columns.append((r[1], r[6], r[8]))
    return columns


def generate(ext, raw):
    gids = {line.split('\t')[0]: i for i, line in enumerate((raw / 'paper/groups.tsv').read_text().splitlines()[1:])}
    paper, vanilla = clay_values(ext / 'paper/clay.tsv'), clay_values(ext / 'vanilla/clay.tsv')
    rows = ['gid\tx\tz\tpaper\tvanilla']
    differ = 0
    for group, x, z in column_set(raw):
        p, v = paper[(group, x, z)], vanilla[(group, x, z)]
        differ += p.split('/')[1] != v.split('/')[1]
        rows.append('\t'.join([str(gids[group]), x, z, p, v]))
    text = '\n'.join(rows) + '\n'
    manifest = {
        'scope': 'GEN-05.9 runtime A5: genuine Paper and Vanilla SurfaceSystem.clayBandsOffsetNoise.getValue(x, 0.0, z) '
                 'recorded by the extended copy of the frozen original helper in every observed genuine column; mechanical '
                 'projection only, no expectation formula',
        'column_set': COLUMN_SET,
        'pass_criterion': CRITERION,
        'counts': {'columns': len(rows) - 1, 'genuine_paper_vanilla_bits_differ': differ},
        'raw_sha256': {'extended/paper/clay.tsv': digest(ext / 'paper/clay.tsv'),
                       'extended/vanilla/clay.tsv': digest(ext / 'vanilla/clay.tsv'),
                       'frozen/paper/rows.tsv': digest(raw / 'paper/rows.tsv'),
                       'frozen/paper/groups.tsv': digest(raw / 'paper/groups.tsv')},
        'extended_run_manifest_sha256': digest(ext / 'manifest.json'),
        'frozen_raw_manifest_sha256': digest(raw / 'manifest.json'),
        'files': {'clay.tsv': hashlib.sha256(text.encode()).hexdigest()},
    }
    return {'clay.tsv': text, 'manifest.json': json.dumps(manifest, sort_keys=True, indent=2) + '\n'}


def route2(ext, raw, fix):
    """Route 2: column set from the raw rows by maximal G/C blocks, then expand every fixture row."""
    groups = [line.split('\t')[0] for line in (raw / 'paper/groups.tsv').read_text().splitlines()[1:]]
    wanted = set()
    for line in (raw / 'paper/rows.tsv').read_text().splitlines()[1:]:
        cells = line.split('\t')
        if cells[0] != 'D' and cells[24].startswith('minecraft:') and cells[24].split(':')[1] in (
                'badlands', 'eroded_badlands', 'wooded_badlands'):
            wanted.add((cells[1], int(cells[6]), int(cells[8])))
    paper, vanilla = clay_values(ext / 'paper/clay.tsv'), clay_values(ext / 'vanilla/clay.tsv')
    listed = set()
    for line in (fix / 'clay.tsv').read_text().splitlines()[1:]:
        gid, x, z, p, v = line.split('\t')
        key = (groups[int(gid)], x, z)
        require(paper[key] == p and vanilla[key] == v, 'clay expansion ' + line)
        listed.add((key[0], int(x), int(z)))
    require(listed == wanted, 'column set differs between routes')
    return len(listed)


def main():
    require(len(sys.argv) == 5 and sys.argv[1] in ('generate', 'validate'), 'generate EXT RAW OUT | validate EXT RAW FIX')
    ext, raw, target = (Path(a).resolve() for a in sys.argv[2:5])
    files = generate(ext, raw)
    if sys.argv[1] == 'generate':
        require(not target.exists(), 'fresh output required')
        target.mkdir()
        for name, text in files.items():
            (target / name).write_bytes(text.encode())
        print(json.dumps({'files': {n: len(t.encode()) for n, t in files.items()}}, sort_keys=True))
        return
    for name, text in files.items():
        require((target / name).read_bytes() == text.encode(), 'regenerated fixture differs: ' + name)
    require(sorted(p.name for p in target.iterdir()) == sorted(files), 'fixture membership')
    columns = route2(ext, raw, target)
    print(json.dumps({'regenerated_equal': True, 'columns_rederived': columns,
                      'counts': json.loads(files['manifest.json'])['counts']}, sort_keys=True))


if __name__ == '__main__':
    main()
