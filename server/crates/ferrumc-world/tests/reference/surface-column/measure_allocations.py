#!/usr/bin/env python3
"""Run the accepted external allocator probe against a built World unit-test binary (GEN-05.10)."""
import argparse
import hashlib
import json
from pathlib import Path
import os
import re
import subprocess
import sys

sys.dont_write_bytecode = True

TEST = 'surface_rules::column::tests::allocation::measured_bind_buffer_and_every_fixture_chunk'
CENSUS = 'measured 1 bind + 1 buffer, 70 published chunks, 70 refused chunks'


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
    if CENSUS not in result.stdout:
        raise RuntimeError('measured operation census changed')
    pattern = r'GEN057_ALLOC phase=(\w+) calls=(\d+) bytes=(\d+) frees=(\d+) stack_write_bytes=(\d+)'
    entries = re.findall(pattern, result.stderr)
    if len(entries) != 3:
        raise RuntimeError('missing measurement phases: ' + result.stderr)
    records = {phase: {'calls': int(calls), 'bytes': int(size), 'frees': int(frees), 'stack_write_bytes': int(stack)}
               for phase, calls, size, frees, stack in entries}
    if set(records) != {'CONTROL', 'BUILD', 'DRIVE'}:
        raise RuntimeError('phase identities changed')
    if records['CONTROL']['calls'] < 1 or records['CONTROL']['bytes'] < 4096:
        raise RuntimeError('positive allocation control failed')
    if any(records['DRIVE'][name] for name in ('calls', 'bytes', 'frees')):
        raise RuntimeError('unexpected allocation: DRIVE ' + str(records['DRIVE']))
    report = {
        'phases': records,
        'operations': CENSUS,
        'binary': str(args.binary.resolve()),
        'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
        'external_probe_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
        'command': command,
        'scope': 'Linux/glibc allocator observations for this exact binary: bind and buffer counted, '
                 'build_chunk (published and refused) must be zero; no RSS, TPS or latency claim.',
    }
    (args.out / 'measurement.json').write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(json.dumps(report['phases'], sort_keys=True))


if __name__ == '__main__':
    main()
