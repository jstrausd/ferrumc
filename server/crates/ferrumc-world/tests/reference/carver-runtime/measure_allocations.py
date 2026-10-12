#!/usr/bin/env python3
"""Run the accepted external allocator probe against a built World unit-test binary (GEN-05.11)."""
import argparse
import hashlib
import json
from pathlib import Path
import os
import re
import subprocess
import sys

sys.dont_write_bytecode = True

TEST = 'carver::tests::allocation::cold_success_and_every_refusal'
CENSUS = 'GEN0511_ALLOC_CENSUS first-cold=1 success=21 published-refusal=21 constructor-refusal=13'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--native', action='store_true')
    parser.add_argument('--test', default=TEST)
    parser.add_argument('--census', default=CENSUS)
    args = parser.parse_args()
    if args.out.exists():
        raise RuntimeError('measurement output must be fresh')
    args.out.mkdir(parents=True)
    # The unchanged GEN-05.7 test-only preload library; never linked into production.
    source = (Path(__file__).resolve().parent.parent / 'biome-temperature' / 'allocator_probe.c').resolve()
    if args.native:
        source = Path(__file__).resolve().parent / 'native_allocator_probe.c'
    library = args.out.resolve() / 'allocator_probe.so'
    command = ['gcc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-shared', '-fPIC', '-O2', '-pthread',
               str(source), '-o', str(library)]
    subprocess.run(command, check=True, capture_output=True)
    # Pin the executable before launch so later development builds cannot alter
    # the measured process's recorded identity.
    import shutil
    binary = args.out.resolve() / 'world-probe'
    shutil.copy2(args.binary.resolve(), binary)
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
    environment = dict(os.environ, LD_PRELOAD=str(library))
    command = [str(binary), '--ignored', '--exact', args.test, '--nocapture', '--test-threads=1']
    result = subprocess.run(command, env=environment, capture_output=True, text=True)
    (args.out / 'stdout.log').write_text(result.stdout)
    (args.out / 'stderr.log').write_text(result.stderr)
    if result.returncode:
        raise RuntimeError('probe test failed')
    if args.census not in result.stdout:
        raise RuntimeError('measured operation census changed')
    pattern = r'GEN057_ALLOC phase=(\w+) calls=(\d+) bytes=(\d+) frees=(\d+) stack_write_bytes=(\d+)'
    entries = re.findall(pattern, result.stderr)
    if len(entries) != 3:
        raise RuntimeError('missing measurement phases: ' + result.stderr)
    records = {phase: {'calls': int(calls), 'bytes': int(size), 'frees': int(frees), 'stack_write_bytes': int(stack)}
               for phase, calls, size, frees, stack in entries}
    if set(records) != {'CONTROL', 'BUILD', 'DRIVE'}:
        raise RuntimeError('phase identities changed')
    if [records['CONTROL'][k] for k in ('calls','bytes','frees')] != [1,4096,1]:
        raise RuntimeError('positive allocation control failed')
    if any(records['DRIVE'][name] for name in ('calls', 'bytes', 'frees')):
        raise RuntimeError('unexpected allocation: DRIVE ' + str(records['DRIVE']))
    native = re.findall(r'phase=(\w+).*native_live_bytes=(\d+) native_live_objects=(\d+) native_peak_bytes=(\d+) native_peak_objects=(\d+)', result.stderr)
    if args.native and len(native) != 3:
        raise RuntimeError('incomplete native census')
    report = {
        'phases': records,
        'native': {phase: dict(zip(('live_bytes','live_objects','peak_bytes','peak_objects'), map(int, values))) for phase, *values in native},
        'operations': args.census,
        'binary': str(binary),
        'binary_sha256': binary_hash,
        'external_probe_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
        'command': command,
        'scope': 'Linux/glibc allocator observations for this exact cold binary: constructors counted, '
                 'prepared success and already-Published refusal calls must be zero allocations/deallocations; no RSS, TPS or latency claim.',
    }
    (args.out / 'measurement.json').write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(json.dumps(report['phases'], sort_keys=True))


if __name__ == '__main__':
    main()
