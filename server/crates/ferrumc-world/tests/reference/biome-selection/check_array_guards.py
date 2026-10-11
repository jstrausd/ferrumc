"""Rejection checks; hash/ZIP faults are declared and leave artifacts unchanged."""
from pathlib import Path
import importlib.util
import json
import os
import sys


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    require(len(sys.argv) == 2, 'fresh receipt path required')
    receipt = Path(sys.argv[1]).resolve()
    require(not receipt.exists(), 'receipt must be fresh')
    runner = Path(__file__).with_name('run_arrays.py').resolve()
    root = next(p for p in runner.parents if (p / 'AGENTS.md').is_file())
    spec = importlib.util.spec_from_file_location('array_guard', runner)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    results = []

    def reject(label, expected):
        target = receipt.with_name(receipt.name + '-' + label)
        sys.argv = [str(runner), str(target)]
        try:
            module.main()
        except ValueError as error:
            require(expected in str(error), 'wrong rejection ' + str(error))
            require(not target.exists(), 'rejection created output')
            results.append({'guard': label, 'rejected': str(error), 'output_absent': True})
            return
        raise ValueError('guard accepted ' + label)

    for name in ('JAVA_TOOL_OPTIONS', '_JAVA_OPTIONS', 'JDK_JAVA_OPTIONS', 'JDK_JAVAC_OPTIONS', 'CLASSPATH'):
        old = os.environ.get(name)
        os.environ[name] = 'declared-invalid-injection'
        try:
            reject(name, 'Java environment injection ' + name)
        finally:
            if old is None:
                os.environ.pop(name, None)
            else:
                os.environ[name] = old
    original_digest = module.digest
    faults = {'source-hash': runner.with_name('BiomeArrayOracle.java'), 'jdk-hash': Path('/home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1/bin/java')}
    for label, path in faults.items():
        module.digest = lambda p, path=path: '0' * 64 if Path(p).resolve() == path.resolve() else original_digest(p)
        try:
            reject(label, 'mismatch')
        finally:
            module.digest = original_digest
    original_zip = module.zipfile.ZipFile

    class DeclaredShadow:
        def __init__(self, path):
            self.archive = original_zip(path)

        def __enter__(self):
            return self

        def __exit__(self, *args):
            self.archive.close()

        def namelist(self):
            return self.archive.namelist() + ['ferrumc/biomearray/BiomeArrayOracle.class']

    module.zipfile.ZipFile = DeclaredShadow
    try:
        reject('declared-artifact-class-shadow', 'helper shadowed by artifact ')
    finally:
        module.zipfile.ZipFile = original_zip
    require(len(results) == 8, 'incomplete guard checks')
    receipt.write_text(json.dumps({'optimization': sys.flags.optimize, 'scope': 'Real main environment guards plus declared hash/ZIP-name fault injection; original artifacts unmodified', 'runner': str(runner.relative_to(root)), 'results': results}, indent=2, sort_keys=True) + '\n')
    print('8 original-array rejection guards passed')


if __name__ == '__main__':
    main()
