"""Fresh original-class execution only; no column-driver numerical expectations."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys
import zipfile
sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
AREA = ROOT / '.codex-tmp/gen05_10'
JAVA_ENV = ('JAVA_TOOL_OPTIONS', '_JAVA_OPTIONS', 'JDK_JAVA_OPTIONS', 'JDK_JAVAC_OPTIONS', 'CLASSPATH')
HELPER_PREFIX = 'ferrumc/surfacecolumnoracle/'
HELPER_CLASSES = {HELPER_PREFIX + n + '.class' for n in (
    'SurfaceColumnOracle', 'SurfaceColumnOracle$Group', 'SurfaceColumnOracle$Gen', 'SurfaceColumnOracle$Snap',
    'SurfaceColumnOracle$ColRec', 'SurfaceColumnOracle$Recording', 'SurfaceColumnOracle$Column', 'SurfaceColumnOracle$Evaluation')}
INNER = {'vanilla': '2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250',
         'paper': 'c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507'}
ARTIFACT_COUNT = {'vanilla': 45, 'paper': 120}
ARCHIVE_FILES = 1386


class GuardFailure(ValueError):
    """No completed output after an invalid authority or environment."""


def require(condition, message):
    if not condition:
        raise GuardFailure(message)


def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1048576), b''):
            h.update(block)
    return h.hexdigest()


def environment_guard(environment):
    for name in JAVA_ENV:
        require(not environment.get(name), 'Java environment injection ' + name)


def inventory_guard(inventory):
    for name, expected in inventory.items():
        require(digest(name) == expected, 'Pinned input mismatch ' + name)


def canonical_guard(source):
    require(Path(source).resolve() == AREA / 'oracle/SurfaceColumnOracle.java', 'Noncanonical helper source')


def shadow_guard(names, artifact):
    require(not any(name.startswith(HELPER_PREFIX) for name in names), 'Helper package shadow ' + artifact)


def output_guard(output):
    require(AREA in output.parents and not output.exists(), 'Output must be fresh inside authorized GEN05.10 area')


def membership_guard(pins):
    base = ROOT / '.codex-tmp/gen05_10-root-base/archive'
    names = {str(p.relative_to(base)) for p in base.rglob('*') if p.is_file()}
    require(names == set(pins['root_archive_manifest']) and len(names) == ARCHIVE_FILES, 'Root archive membership changed')
    current = {str(p) for p in Path(pins['jdk']).rglob('*') if p.is_file()}
    require(current == set(pins['jdk_inventory']), 'Complete JDK membership changed')
    for folder in ('design', 'oracle', 'sources'):
        expected = {name for name in pins['inventory'] if name.startswith(str(AREA / folder) + '/')}
        current = {str(p) for p in (AREA / folder).rglob('*') if p.is_file() and p.name != 'input-pins.json' and '__pycache__' not in p.parts}
        require(current == expected, 'Preparation source/input membership changed: ' + folder)


def original_inner(pins, mode):
    jars = pins['runtime_inputs'][mode]
    require(len(jars) == ARTIFACT_COUNT[mode], 'Original artifact count changed')
    matches = [name for name, value in jars.items() if value == INNER[mode]]
    require(len(matches) == 1, 'Unique pinned original inner required')
    return matches[0]


def preflight(pins):
    environment_guard(os.environ)
    canonical_guard(HERE / 'SurfaceColumnOracle.java')
    require(str(Path(__file__).resolve()) in pins['inventory'], 'Canonical runner must be pinned')
    inventory_guard(pins['inventory'])
    membership_guard(pins)
    for mode in ('vanilla', 'paper'):
        original_inner(pins, mode)
        for name in pins['runtime_inputs'][mode]:
            with zipfile.ZipFile(name) as jar:
                shadow_guard(jar.namelist(), name)


def classpath(pins, mode, classes):
    inner = original_inner(pins, mode)
    jars = [name for name in pins['runtime_inputs'][mode] if name != inner and not name.endswith('/server.jar')]
    return inner, os.pathsep.join([inner, *sorted(jars), str(classes)])


def java_args(pins, run, out, inner, classes, selected):
    mode = 'paper' if run == 'scan' else run
    return [run, str(out), inner, str(classes), str(HERE / 'semantic-mappings.properties'), str(AREA / 'design'),
            str(HERE / (mode + '-library-origins.properties')), str(HERE / 'aliases.txt'), str(HERE / 'paper-aliases.txt'), str(selected)]


def main():
    require(len(sys.argv) == 2, 'One fresh output directory required')
    output = Path(sys.argv[1]).resolve()
    output_guard(output)
    pins_path = HERE / 'input-pins.json'
    pins = json.loads(pins_path.read_text())
    pin_hash = digest(pins_path)
    preflight(pins)
    environment = dict(os.environ)
    for name in JAVA_ENV:
        environment.pop(name, None)
    environment['PYTHONDONTWRITEBYTECODE'] = '1'
    output.mkdir()
    classes = output / 'classes'
    classes.mkdir()
    jdk = Path(pins['jdk'])
    subprocess.run([str(jdk / 'bin/javac'), '-proc:none', '-encoding', 'UTF-8', '-Xlint:-unchecked', '-d', str(classes),
                    str(HERE / 'SurfaceColumnOracle.java')], env=environment, check=True)
    emitted = {str(p.relative_to(classes)) for p in classes.rglob('*') if p.is_file()}
    require(emitted == HELPER_CLASSES, 'Unexpected freshly compiled helper inventory ' + str(sorted(emitted)))
    selected = output / 'scan' / 'selected.tsv'
    for run in ('scan', 'vanilla', 'paper'):
        mode = 'paper' if run == 'scan' else run
        inner, cp = classpath(pins, mode, classes)
        run_output = output / run
        run_output.mkdir()
        with (output / (run + '.log')).open('w') as log:
            subprocess.run([str(jdk / 'bin/java'), '-Xmx3G', '-Dmax.bg.threads=1', '-cp', cp, 'ferrumc.surfacecolumnoracle.SurfaceColumnOracle',
                            *java_args(pins, run, run_output, inner, classes, selected)],
                           cwd=output, env=environment, stdout=log, stderr=subprocess.STDOUT, check=True)
        require(selected.is_file(), 'Genuine scan selection missing')
    preflight(pins)
    require(digest(pins_path) == pin_hash, 'Input manifest changed during export')
    sys.path.insert(0, str(HERE))
    from project_originals import validate_and_project
    report = validate_and_project(output, pins)
    all_files = {str(p.relative_to(output)): digest(p) for p in output.rglob('*') if p.is_file()}
    diagnostic_files = {name: value for name, value in all_files.items() if name.endswith('.log') or name.startswith('logs/') or '/logs/' in name}
    files = {name: value for name, value in all_files.items() if name not in diagnostic_files}
    (output / 'diagnostics-manifest.json').write_text(json.dumps({
        'scope': 'Preserved timestamped original bootstrap/stdio logs; not differential fixture expectations',
        'outputs': diagnostic_files}, sort_keys=True, indent=2) + '\n')
    manifest = {'scope': 'Actual original NoiseBasedChunkGenerator/SurfaceSystem/ProtoChunk/Heightmap/BiomeManager/NoiseChunk objects; '
                         'genuine createBiomes, fillFromNoise and buildSurface; no WorldGenRegion construction, carver, feature, '
                         'ServerLevel tick or server execution',
                'input_pins_sha256': pin_hash, 'input_inventory': pins['inventory'], 'outputs': files, 'report': report,
                'pre_post_whole_input_bytes_and_membership_unchanged': True}
    (output / 'manifest.json').write_text(json.dumps(manifest, sort_keys=True, indent=2) + '\n')
    print(json.dumps({'groups': {m: report.get(m, {}).get('groups') for m in ('vanilla', 'paper')},
                      'model_mismatches': {m: report.get(m, {}).get('model_mismatch_count') for m in ('vanilla', 'paper')}}, sort_keys=True))


if __name__ == '__main__':
    main()
