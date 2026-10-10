"""Material provenance refusals, without mutating shared JAR/source inputs."""
from pathlib import Path
import json
import os
import shutil
import subprocess
import sys
import zipfile
import generate_material_goldens as runner


def refuses(label, action, expected, results):
    try:
        action()
    except ValueError as error:
        runner.require(expected in str(error), 'wrong refusal for ' + label + ': ' + str(error))
        results.append({'case': label, 'refused': str(error)})
    else:
        raise ValueError('guard accepted ' + label)


def main():
    runner.require(len(sys.argv) == 3, 'VALID_EXPORT FRESH_TEST_DIR required')
    export, output = map(lambda value: Path(value).resolve(), sys.argv[1:])
    output.mkdir(exist_ok=False)
    inner, paths, artifacts, _ = runner.configuration()
    results = []
    for key in runner.ENV:
        refuses('env-' + key, lambda: runner.reject_env({key: '-javaagent:untrusted.jar'}), key, results)
    altered = output / 'altered.bin'
    altered.write_bytes(b'intentionally different reference bytes\n')
    for label, expected in runner.PIN.items():
        refuses('hash-' + label, lambda: runner.verify(altered, expected, label), label + ' hash mismatch', results)
    for relative, expected in runner.JDK_PINS.items():
        refuses('hash-jdk-' + relative, lambda: runner.verify(altered, expected, relative), relative + ' hash mismatch', results)
    library = next(path for path in paths if path != inner)
    refuses('hash-library', lambda: runner.verify(altered, paths[library], 'library'), 'library hash mismatch', results)
    for suffix in ('', '$Shadow'):
        shadow = output / ('shadow' + suffix.replace('$', '-') + '.jar')
        with zipfile.ZipFile(shadow, 'w') as archive:
            archive.writestr('ferrumc/rootmaterial/ChunkMaterialGoldens' + suffix + '.class', b'shadow bytes')
        refuses('shadow' + suffix, lambda: runner.reject_shadow([shadow]), 'helper classpath shadow', results)
    classes = export / 'classes'
    runner.require(classes.is_dir(), 'fresh actual helper classfiles missing')
    wrong_classes = output / 'wrong-classes'
    shutil.copytree(classes, wrong_classes)
    for label in ('original-origin', 'helper-origin'):
        target = output / label
        target.mkdir()
        # The actual helper expects its fresh CodeSource under target/classes.
        # Only the helper-origin case deliberately keeps another classdirectory.
        actual_classes = classes if label == 'helper-origin' else target / 'classes'
        if label == 'helper-origin':
            (target / 'classes').mkdir()
        else:
            shutil.copytree(classes, actual_classes)
        cp = os.pathsep.join([inner, *sorted(path for path in paths if path != inner), str(actual_classes)])
        expected_inner = str(wrong_classes) if label == 'original-origin' else inner
        process = subprocess.run([
            str(runner.JAVA_HOME / 'bin/java'), '-Xmx256M', '-cp', cp,
            'ferrumc.rootmaterial.ChunkMaterialGoldens', str(runner.FIXTURE), str(target), expected_inner,
        ], capture_output=True, text=True, cwd=target)
        log = process.stdout + process.stderr
        expected = 'wrong original origin' if label == 'original-origin' else 'wrong fresh helper origin'
        runner.require(process.returncode != 0 and expected in log, 'actual JVM guard accepted ' + label)
        runner.require(not (target / 'chunks.jsonl').exists() and not (target / 'material_states.bits').exists(),
                       'actual origin refusal published material output')
        (output / (label + '.log')).write_text(log)
        results.append({'case': label, 'refused': expected, 'output_absent': True})
    for path, expected in artifacts.items():
        runner.verify(path, expected, 'changed input after guard checks ' + path)
    (output / 'results.json').write_text(json.dumps({'cases': results, 'count': len(results), 'python_optimized': not __debug__}, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'guards_passed': len(results), 'python_optimized': not __debug__}))


if __name__ == '__main__':
    main()
