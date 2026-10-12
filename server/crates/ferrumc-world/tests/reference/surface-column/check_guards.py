"""Explicit normal/-O guards; fault injection never edits original inputs."""
from pathlib import Path
import copy
import json
import os
import subprocess
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from run_oracle import (AREA, HERE, JAVA_ENV, HELPER_PREFIX, GuardFailure, canonical_guard, classpath, digest,
                        environment_guard, inventory_guard, java_args, membership_guard, output_guard, preflight, require,
                        shadow_guard)


def main():
    require(len(sys.argv) == 3, 'Fresh guard output and completed reference output required')
    out = Path(sys.argv[1]).resolve()
    reference = Path(sys.argv[2]).resolve()
    output_guard(out)
    pins = json.loads((HERE / 'input-pins.json').read_text())
    preflight(pins)
    require((reference / 'manifest.json').is_file(), 'Completed original reference required')
    out.mkdir()
    results = []

    def refused(name, kind, operation, message):
        try:
            operation()
        except GuardFailure as e:
            require(message in str(e), 'Wrong guard refusal ' + name)
            results.append({'guard': name, 'kind': kind, 'result': 'refused', 'reason': message})
        else:
            raise GuardFailure('Guard silently admitted ' + name)

    for name in JAVA_ENV:
        refused('environment-' + name, 'actual supplied environment', lambda n=name: environment_guard({n: 'injected'}), 'Java environment injection')
        env = dict(os.environ)
        env[name] = 'injected'
        candidate = out / ('blocked-' + name)
        # The real entrypoint inherits this guard run's interpreter optimization mode.
        flags = ['-' + 'O' * sys.flags.optimize] if sys.flags.optimize else []
        process = subprocess.run([sys.executable, *flags, str(HERE / 'run_oracle.py'), str(candidate)], env=env,
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        require(process.returncode != 0 and 'Java environment injection' in process.stderr and not candidate.exists(),
                'Actual entrypoint environment guard ' + name)
    source = HERE / 'SurfaceColumnOracle.java'
    refused('canonical-source', 'actual alternate path', lambda: canonical_guard(out / 'SurfaceColumnOracle.java'), 'Noncanonical helper source')
    refused('digest', 'declared wrong expected hash; actual pinned file unchanged', lambda: inventory_guard({str(source): '0' * 64}), 'Pinned input mismatch')
    refused('helper-package-shadow', 'declared ZIP namelist fault; original JAR unchanged', lambda: shadow_guard([HELPER_PREFIX + 'Other.class'], 'declared-list'), 'Helper package shadow')
    refused('fresh-output', 'actual existing directory', lambda: output_guard(out), 'Output must be fresh')
    refused('write-scope', 'actual outside path, no write attempted', lambda: output_guard(AREA.parent / 'refused-gen05_10-output'), 'Output must be fresh')
    drift = copy.deepcopy(pins)
    drift['jdk_inventory'][pins['jdk'] + '/not-an-original-file'] = '0' * 64
    refused('jdk-membership', 'declared manifest membership fault', lambda: membership_guard(drift), 'Complete JDK membership changed')
    drift = copy.deepcopy(pins)
    drift['root_archive_manifest']['not-an-archive-file'] = '0' * 64
    refused('archive-membership', 'declared manifest membership fault', lambda: membership_guard(drift), 'Root archive membership changed')
    drift = copy.deepcopy(pins)
    drift['inventory'][str(AREA / 'design/not-a-declared-input.tsv')] = '0' * 64
    refused('design-membership', 'declared manifest membership fault', lambda: membership_guard(drift), 'Preparation source/input membership changed: design')
    # Real JVM origin refusals use freshly compiled pinned helper classes. They fail before
    # bootstrap, generation or any output and preserve every original.
    mode = 'vanilla'
    classes = reference / 'classes'
    inner, cp = classpath(pins, mode, classes)
    env = dict(os.environ)
    for n in JAVA_ENV:
        env.pop(n, None)
    env['PYTHONDONTWRITEBYTECODE'] = '1'
    prop = out / 'wrong-library.properties'
    prop.write_text('com.google.gson.JsonParser=' + str(classes) + '\n')
    controls = [('original-origin', 2, str(classes), 'Wrong actual CodeSource original/'),
                ('helper-origin', 3, str(out), 'Wrong actual CodeSource helper/main'),
                ('library-origin', 6, str(prop), 'Wrong actual CodeSource library/')]
    for name, index, value, reason in controls:
        target = out / name
        target.mkdir()
        args = java_args(pins, mode, target, inner, classes, reference / 'scan/selected.tsv')
        args[index] = value
        proc = subprocess.run([str(Path(pins['jdk']) / 'bin/java'), '-Dmax.bg.threads=1', '-cp', cp, 'ferrumc.surfacecolumnoracle.SurfaceColumnOracle', *args],
                              env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (out / (name + '.log')).write_text(proc.stdout)
        require(proc.returncode != 0 and reason in proc.stdout, 'Wrong actual origin refusal ' + name)
        require(not any(target.iterdir()), 'Origin guard emitted sample data ' + name)
        results.append({'guard': name, 'kind': 'actual JVM CodeSource mismatch', 'result': 'refused-before-samples', 'reason': reason})
    preflight(pins)
    report = {'scope': 'Input/environment/CodeSource guards; no FerrumC runtime-policy acceptance', 'guards': results, 'count': len(results),
              'normal_and_optimized_use_explicit_require': True, 'original_inputs_unchanged': True,
              'entrypoint_python_optimize': sys.flags.optimize}
    (out / 'guards.json').write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
    print(json.dumps({'guards': len(results), 'all_refused': True, 'guards_sha256': digest(out / 'guards.json')}, sort_keys=True))


if __name__ == '__main__':
    main()
