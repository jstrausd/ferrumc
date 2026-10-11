#!/usr/bin/env python3
"""Mechanical import of approved original outputs; no numerical expectations.

Run from the repository holding the immutable .codex-tmp/gen05_7 authority.
The output must be fresh. All original inputs/evidence are hashed pre/post.
Only enormous cache FIFO strings are omitted; their exact original files,
selected observations and all differences remain pinned in provenance.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

PINS = {
    '.codex-tmp/gen05_7/oracle.freeze.json': 'e785007c594f1be18ce957c23f70652ab6a01da8fa80582eb3fec703572371bd',
    '.codex-tmp/gen05_7/design.freeze.json': 'f195f5c080335b9118083b60c3325f721b03123cd1f89f0d87f764b2f03de013',
    '.codex-tmp/gen05_7/oracle/input-pins.json': 'cae06b94b446ce6c07f644541b2b7c689c34e92d4a8d29bf01368981b206c9a8',
    'docs/agent-tasks/GEN-05_7_BIOME_TEMPERATURE.md': 'be5b6f86ba846af690cf81f10b0b24b864796fadd44e63b0ce92c0906122d7a8',
    '.codex-tmp/gen05_7/oracle/extension-contract.json': 'cacfd7cdb7bfda05e05bb6e9c3f6ad67e138cd934d909328f58a8901b0310c49',
    '.codex-tmp/gen05_7/oracle/frozen-extension-inputs.tsv': '8e5f79b25405651f5d2d3989b159a8dbfcc75e060f0b4d61c29e4c58ed3ce5c0',
}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        while block := stream.read(1024 * 1024):
            h.update(block)
    return h.hexdigest()


def load(path):
    return json.loads(path.read_text())


def audit(repo):
    for name, expected in PINS.items():
        require(digest(repo / name) == expected, 'authority drift: ' + name)
    evidence = load(repo / '.codex-tmp/gen05_7/oracle.freeze.json')['files']
    design = load(repo / '.codex-tmp/gen05_7/design.freeze.json')['files']
    actual = load(repo / '.codex-tmp/gen05_7/oracle/input-pins.json')['inventory']
    require((len(evidence), len(design), len(actual)) == (119, 64, 757), 'authority census drift')
    observed = {}
    for inventory in (evidence, design, actual):
        for name, pin in inventory.items():
            expected = pin['sha256'] if isinstance(pin, dict) else pin
            path = Path(name) if Path(name).is_absolute() else repo / name
            found = digest(path)
            require(found == expected, 'input drift: ' + name)
            observed[name] = found
    return observed


def lines(path):
    return [line.split('\t') for line in path.read_text().splitlines()]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    require(not args.out.exists(), 'output must be fresh')
    before = audit(args.repo)
    args.out.mkdir(parents=True)
    original = args.repo / '.codex-tmp/gen05_7/final-original-normal'
    provenance = {'authority': PINS, 'original_outputs': {}, 'imported_outputs': {},
                  'omitted': 'Only full cache FIFO strings; original cache.tsv hashes and all diagnostic differences retained.',
                  'expectations': 'Only actual original output tokens; no arithmetic, normalization or substituted bits.',
                  'rows': {}}
    with (args.out / 'temperatures.tsv').open('w') as output:
        for filename, kind, count in [('scalar.tsv', 'S', 93584), ('mutable.tsv', 'M', 80),
                                     ('cache.tsv', 'C', 3156), ('frozen-extension.tsv', 'F', 32768)]:
            modes = [lines(original / mode / filename) for mode in ('vanilla', 'paper')]
            require(all(len(rows) == count for rows in modes), 'count drift: ' + filename)
            for vanilla, paper in zip(*modes, strict=True):
                require(vanilla[:6] == paper[:6], 'case identity drift')
                if kind == 'C':
                    vanilla = vanilla[:-1]
                    paper = paper[:-1]
                output.write('\t'.join([kind] + vanilla + paper[6:]) + '\n')
            provenance['rows'][kind] = count
    for filename in ('metadata.tsv', 'noise.tsv'):
        require((original / 'vanilla' / filename).read_bytes() ==
                (original / 'paper' / filename).read_bytes(), 'shared original output differs: ' + filename)
        shutil.copyfile(original / 'vanilla' / filename, args.out / filename)
    for mode in ('vanilla', 'paper'):
        shutil.copyfile(original / mode / 'constructors.tsv', args.out / (mode + '-constructors.tsv'))
        shutil.copyfile(original / mode / 'origins.tsv', args.out / (mode + '-origins.tsv'))
        for path in sorted((original / mode).glob('*.tsv')):
            provenance['original_outputs'][mode + '/' + path.name] = {'bytes': path.stat().st_size, 'sha256': digest(path)}
    shutil.copyfile(args.repo / '.codex-tmp/gen05_7/oracle-comparison.json', args.out / 'diagnostics.json')
    # Exact copies preserve original canonical helper/runner/guard pinning and
    # refusal semantics. Reproduction runs those immutable canonical paths.
    sources = args.out / 'authority'
    for name in ['.codex-tmp/gen05_7/oracle.freeze.json', '.codex-tmp/gen05_7/design.freeze.json',
                 '.codex-tmp/gen05_7/census/metadata.json',
                 'docs/agent-tasks/GEN-05_7_BIOME_TEMPERATURE.md',
                 'docs/agent-tasks/GEN-05_7_BIOME_TEMPERATURE_CONTRACT_REVIEW.md',
                 'docs/agent-tasks/GEN-05_7_BIOME_TEMPERATURE_ORACLE_REVIEW.md']:
        path = sources / name
        path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(args.repo / name, path)
    for folder, names in {
        'oracle': ['TemperatureOracle.java', 'run_oracle.py', 'check_guards.py', 'prepare_inputs.py', 'prepare_extension.py',
                   'input-pins.json', 'semantic-mappings.properties', 'extension-contract.json',
                   'paper-library-origins.properties', 'vanilla-library-origins.properties'],
        'design': ['cases.json', 'build_case_contract.py'],
    }.items():
        for name in names:
            relative = '.codex-tmp/gen05_7/' + folder + '/' + name
            target = sources / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(args.repo / relative, target)
    require(before == audit(args.repo), 'pre/post authority changed')
    provenance['pre_post_unchanged'] = {'evidence': 119, 'design': 64, 'actual_inputs': 757}
    for path in sorted(args.out.rglob('*')):
        if path.is_file():
            provenance['imported_outputs'][path.relative_to(args.out).as_posix()] = {'bytes': path.stat().st_size, 'sha256': digest(path)}
    (args.out / 'provenance.json').write_text(json.dumps(provenance, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'rows': provenance['rows'], 'imported_files': len(provenance['imported_outputs']), 'authority_verified': provenance['pre_post_unchanged']}))


if __name__ == '__main__':
    main()
