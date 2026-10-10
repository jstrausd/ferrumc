from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys
import zipfile


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(65536), b''):
            h.update(chunk)
    return h.hexdigest()


def main():
    require(len(sys.argv) == 2, 'one fresh output path required')
    root = Path(__file__).resolve().parents[2]
    output = Path(sys.argv[1]).resolve()
    require(not output.exists(), 'output must be fresh')
    source = Path(__file__).with_name('BiomeParameterCensus.java')
    inputs_path = root / '.codex-tmp/gen05_2-root-paper-end/inputs.json'
    inputs = json.loads(inputs_path.read_text())['paper']
    require(len(inputs) == 120, 'exact Paper inner +119 libraries required')
    candidates = [path for path, pin in inputs.items() if pin == 'c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507']
    require(len(candidates) == 1, 'unique original Paper inner required')
    inner = candidates[0]
    for name in ('JAVA_TOOL_OPTIONS', '_JAVA_OPTIONS', 'JDK_JAVA_OPTIONS'):
        require(not os.environ.get(name), 'Java option injection ' + name)
    jdk = Path('/home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1')
    applied = root / '.codex-tmp/paper-1.21.8-reference/paper-server/src/minecraft/java/net/minecraft/world/level/biome'
    base = root / '.codex-tmp/paper-1.21.8-reference/paper-server/.gradle/caches/paperweight/mache/base/sources/net/minecraft/world/level/biome'
    artifacts = dict(inputs)
    for path in (source, Path(__file__).resolve(), inputs_path, jdk/'bin/java', jdk/'bin/javac', jdk/'lib/modules'):
        artifacts[str(path)] = digest(path)
    for directory in (base, applied):
        for name in ('MultiNoiseBiomeSourceParameterList.java', 'OverworldBiomeBuilder.java', 'Climate.java'):
            path = directory / name
            artifacts[str(path)] = digest(path)
    for path, pin in artifacts.items():
        require(digest(path) == pin, 'input hash mismatch ' + path)
    for path in inputs:
        with zipfile.ZipFile(path) as archive:
            require('ferrumc/rootbiomes/BiomeParameterCensus.class' not in archive.namelist(), 'helper shadowed ' + path)
    output.mkdir()
    classes = output / 'classes'
    classes.mkdir()
    cp = os.pathsep.join([inner, *sorted(path for path in inputs if path != inner), str(classes)])
    subprocess.run([str(jdk/'bin/javac'), '-proc:none', '-encoding', 'UTF-8', '-cp', cp, '-d', str(classes), str(source)], check=True)
    with (output / 'census.log').open('w') as stream:
        subprocess.run([str(jdk/'bin/java'), '-Xmx512M', '-cp', cp, 'ferrumc.rootbiomes.BiomeParameterCensus', str(output), inner], cwd=output, stdout=stream, stderr=subprocess.STDOUT, check=True)
    for path, pin in artifacts.items():
        require(digest(path) == pin, 'input changed after export ' + path)
    rows = json.loads((output/'parameters.json').read_text())
    require(set(rows) == {'minecraft:nether', 'minecraft:overworld'}, 'unexpected presets')
    require(len(rows['minecraft:nether']) == 5, 'unexpected Nether count')
    summary = {}
    for preset, entries in rows.items():
        require(bool(entries), 'empty preset')
        for index, entry in enumerate(entries):
            require(entry['index'] == index and len(entry['axes']) == 6, 'wrong original order/shape')
            require(all(len(axis) == 2 and all(type(value) is int for value in axis) and axis[0] <= axis[1] for axis in entry['axes']), 'invalid axis data')
            require(type(entry['offset']) is int, 'invalid offset')
        summary[preset] = {'parameters':len(entries), 'unique_biomes':len({entry['biome'] for entry in entries})}
    manifest = {'inputs':artifacts, 'parameters_sha256':digest(output/'parameters.json'), 'original_class_origins_checked':19, 'fresh_helper_origin_checked':True, 'presets':summary, 'scope':'Actual Paper builtin biome parameter data only; no Rust selector, surface or gameplay acceptance'}
    (output/'manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
    print(json.dumps({'presets':summary, 'parameters_sha256':manifest['parameters_sha256']}))


if __name__ == '__main__':
    main()
