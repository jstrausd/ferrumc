#!/usr/bin/env python3
"""Compare the extended runs with the frozen round-2 raw outputs; clay.tsv is the only addition."""
import hashlib, json, sys
from pathlib import Path
sys.dont_write_bytecode = True
C = Path(__file__).resolve().parent
FROZEN = C.parents[1] / 'gen05_9/round2/oracle-normal'
def h(p): return hashlib.sha256(p.read_bytes()).hexdigest()
frozen = json.loads((FROZEN / 'manifest.json').read_text())
report = {'frozen_manifest_sha256': h(FROZEN / 'manifest.json'), 'runs': {}}
for run in ('oracle-normal', 'oracle-optimized'):
    out = C / run
    m = json.loads((out / 'manifest.json').read_text())
    fo, eo = frozen['outputs'], m['outputs']
    extra = sorted(set(eo) - set(fo)); missing = sorted(set(fo) - set(eo))
    differ = sorted(k for k in set(fo) & set(eo) if fo[k] != eo[k])
    # every frozen differential output re-hashed from disk on both sides
    disk = sorted(k for k in fo if (FROZEN / k).is_file() and (out / k).is_file() and h(FROZEN / k) != h(out / k))
    report['runs'][run] = {'frozen_outputs': len(fo), 'equal': len(set(fo) & set(eo)) - len(differ), 'differ': differ,
                           'missing': missing, 'extra': extra, 'disk_differ': disk,
                           'report_equal': m['report'] == frozen['report'],
                           'input_pins_equal': m['input_pins_sha256'] == frozen['input_pins_sha256'],
                           'clay_sha256': {mode: h(out / mode / 'clay.tsv') for mode in ('vanilla', 'paper')}}
a, b = (C / 'oracle-normal'), (C / 'oracle-optimized')
report['normal_equals_optimized'] = all(h(a / k) == h(b / k) for k in json.loads((a / 'manifest.json').read_text())['outputs'])
# Only the freshly compiled helper classes may differ: the helper is the declared extended copy.
HELPER = 'classes/ferrumc/surfaceruleoracle/'
for r in report['runs'].values():
    r['differ_outside_helper_classes'] = [k for k in r['differ'] + r['disk_differ'] if not k.startswith(HELPER)]
    r['data_outputs_equal'] = r['frozen_outputs'] - len([k for k in r['differ'] if k.startswith(HELPER)])
ok = report['normal_equals_optimized'] and all(not r['differ_outside_helper_classes'] and not r['missing'] and r['extra'] == ['paper/clay.tsv', 'vanilla/clay.tsv'] and r['report_equal'] and r['input_pins_equal'] for r in report['runs'].values())
report['pass'] = ok
print(json.dumps(report, indent=1, sort_keys=True))
sys.exit(0 if ok else 1)
