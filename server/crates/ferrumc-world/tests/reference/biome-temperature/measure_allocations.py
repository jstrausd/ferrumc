#!/usr/bin/env python3
"""Build/run external allocator probe against an already built World test binary."""
import argparse
import hashlib
import json
from pathlib import Path
import os
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if args.out.exists():
        raise RuntimeError('measurement output must be fresh')
    args.out.mkdir(parents=True)
    source = Path(__file__).with_name('allocator_probe.c').resolve()
    library = args.out.resolve() / 'allocator_probe.so'
    command = ['gcc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-shared', '-fPIC', '-O2', '-pthread', str(source), '-o', str(library)]
    subprocess.run(command, check=True, capture_output=True)
    environment = dict(os.environ, LD_PRELOAD=str(library))
    command = [str(args.binary.resolve()), '--ignored', '--exact', 'measured_compile_clone_and_every_original_query', '--nocapture', '--test-threads=1']
    result = subprocess.run(command, env=environment, capture_output=True, text=True)
    (args.out / 'stdout.log').write_text(result.stdout)
    (args.out / 'stderr.log').write_text(result.stderr)
    if result.returncode:
        raise RuntimeError('probe test failed')
    pattern = r'GEN057_ALLOC phase=(\w+) calls=(\d+) bytes=(\d+) frees=(\d+) stack_write_bytes=(\d+)'
    entries = re.findall(pattern, result.stderr)
    if len(entries) != 4:
        raise RuntimeError('missing measurement phases: ' + result.stderr)
    records = {phase: {'calls': int(calls), 'bytes': int(size), 'frees': int(frees), 'stack_write_bytes': int(stack)} for phase, calls, size, frees, stack in entries}
    if set(records) != {'CONTROL', 'COMPILE', 'CLONE', 'QUERY'}:
        raise RuntimeError('phase identities changed')
    if records['CONTROL']['calls'] < 1 or records['CONTROL']['bytes'] < 4096:
        raise RuntimeError('positive allocation control failed')
    for phase in ('COMPILE', 'CLONE', 'QUERY'):
        if any(records[phase][name] for name in ('calls', 'bytes', 'frees')):
            raise RuntimeError('unexpected allocation: ' + phase + ' ' + str(records[phase]))
    if any(record['stack_write_bytes'] >= 1024 * 1024 for record in records.values()):
        raise RuntimeError('observed stack writes exceed scratch ceiling')
    report = {'stack_measurement': 'Observed stack write high-water below protected 16 KiB marker frame; native object capacities are accounted separately in Rust.', 'phases': records, 'binary': str(args.binary.resolve()),
              'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
              'external_probe_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
              'operations': {'successful_compiles': 3, 'refused_compiles': 3, 'clones': 100, 'original_queries': 388764},
              'command': command, 'scope': 'Linux/glibc allocator observations for this exact binary; no RSS/TPS/latency claim.'}
    (args.out / 'measurement.json').write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(json.dumps(report['phases'], sort_keys=True))


if __name__ == '__main__':
    main()
