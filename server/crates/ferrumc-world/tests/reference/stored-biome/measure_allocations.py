#!/usr/bin/env python3
"""Run the accepted external allocator probe against a built World unit-test binary."""
import argparse
import hashlib
import json
from pathlib import Path
import os
import re
import subprocess
import sys

sys.dont_write_bytecode = True

TEST = 'stored_biome::tests::allocation::measured_build_clone_and_every_original_query'
OPERATIONS = {'admitted_builds': 67, 'refused_builds': 60, 'copy_rounds': 100, 'corpus_queries': 532_576}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if args.out.exists():
        raise RuntimeError('measurement output must be fresh')
    args.out.mkdir(parents=True)
    # The unchanged GEN-05.7 test-only preload library; never linked into production.
    source = (Path(__file__).resolve().parent.parent / 'biome-temperature' / 'allocator_probe.c').resolve()
    library = args.out.resolve() / 'allocator_probe.so'
    command = ['gcc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-shared', '-fPIC', '-O2', '-pthread',
               str(source), '-o', str(library)]
    subprocess.run(command, check=True, capture_output=True)
    environment = dict(os.environ, LD_PRELOAD=str(library))
    command = [str(args.binary.resolve()), '--ignored', '--exact', TEST, '--nocapture', '--test-threads=1']
    result = subprocess.run(command, env=environment, capture_output=True, text=True)
    (args.out / 'stdout.log').write_text(result.stdout)
    (args.out / 'stderr.log').write_text(result.stderr)
    if result.returncode:
        raise RuntimeError('probe test failed')
    if 'measured 67 admitted + 60 refused builds, 100 copy rounds, 532576 corpus queries' not in result.stdout:
        raise RuntimeError('measured operation census changed')
    pattern = r'GEN057_ALLOC phase=(\w+) calls=(\d+) bytes=(\d+) frees=(\d+) stack_write_bytes=(\d+)'
    entries = re.findall(pattern, result.stderr)
    if len(entries) != 4:
        raise RuntimeError('missing measurement phases: ' + result.stderr)
    records = {phase: {'calls': int(calls), 'bytes': int(size), 'frees': int(frees), 'stack_write_bytes': int(stack)}
               for phase, calls, size, frees, stack in entries}
    if set(records) != {'CONTROL', 'BUILD', 'CLONE', 'QUERY'}:
        raise RuntimeError('phase identities changed')
    if records['CONTROL']['calls'] < 1 or records['CONTROL']['bytes'] < 4096:
        raise RuntimeError('positive allocation control failed')
    for phase in ('BUILD', 'CLONE', 'QUERY'):
        if any(records[phase][name] for name in ('calls', 'bytes', 'frees')):
            raise RuntimeError('unexpected allocation: ' + phase + ' ' + str(records[phase]))
    report = {
        'phases': records,
        'operations': OPERATIONS,
        'binary': str(args.binary.resolve()),
        'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
        'external_probe_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
        'command': command,
        'stack_measurement': 'Observed stack-write high water below the protected 16 KiB marker frame; '
                             'native object capacities are accounted separately in Rust.',
        'scope': 'Linux/glibc allocator observations for this exact binary; no RSS/TPS/latency claim.',
    }
    (args.out / 'measurement.json').write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(json.dumps(report['phases'], sort_keys=True))


if __name__ == '__main__':
    main()
