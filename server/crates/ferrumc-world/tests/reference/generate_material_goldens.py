"""Fresh actual-Paper material exports; provenance guards also run under -O."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = Path(__file__).resolve().parents[5]
JAVA_HOME = Path('/home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1')
HELPER = HERE / 'ChunkMaterialGoldens.java'
SOURCE_PINS = HERE / 'material-source-pins.json'
ROOT_INPUTS = ROOT / '.codex-tmp/gen05_2-root-paper-end/inputs.json'
FIXTURE = ROOT / 'docs/fixtures/minecraft-1.21.8-original/worldgen.json'
ENV = ('JAVA_TOOL_OPTIONS', '_JAVA_OPTIONS', 'JDK_JAVA_OPTIONS', 'JDK_JAVAC_OPTIONS', 'CLASSPATH')
PIN = {
    'helper': '8e8b2c0a2cc965cc17cc8071142fe030a972c697d99dd582cdd544d8d66c68fb',
    'source_pins': '6619af5dc820d889f1bc13f650eb070c699b0d8e76231e67d68c7a2b9be447bf',
    'root_inputs': 'd7eff61da418e8a8883f69dd2b079cc5c5b1a91612f16745f3c07ec90ff58841',
    'fixture': 'bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30',
    'paper_inner': 'c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507',
    'paper_bundle': '8de7c52c3b02403503d16fac58003f1efef7dd7a0256786843927fa92ee57f1e',
}
JDK_PINS = {
    'bin/java': '2a207f5e7d075afa01d97f8048389a64432a44c4a5af0f5e77d6e286ec5f401d',
    'bin/javac': '55859b80e7a9c4c4736be19ad3addeb35112ca6d17a30c4e0e116afc0a499bdb',
    'lib/modules': '70c5465dabf8e1bec2860631ea3ffd342f0ec23a5e2efd3a389d2abe32448f98',
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    value = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(65536), b''):
            value.update(chunk)
    return value.hexdigest()


def verify(path, expected, label):
    require(digest(path) == expected, label + ' hash mismatch')


def reject_env(environment):
    for name in ENV:
        require(not environment.get(name), 'Java environment injection ' + name)


def reject_shadow(paths):
    for path in paths:
        with zipfile.ZipFile(path) as archive:
            require(not any(name.startswith('ferrumc/rootmaterial/ChunkMaterialGoldens') and name.endswith('.class') for name in archive.namelist()),
                    'helper classpath shadow ' + str(path))


def configuration():
    reject_env(os.environ)
    artifacts = {}
    for label, path in [('helper', HELPER), ('source_pins', SOURCE_PINS), ('root_inputs', ROOT_INPUTS), ('fixture', FIXTURE)]:
        verify(path, PIN[label], label)
        artifacts[str(path)] = PIN[label]
    sources = json.loads(SOURCE_PINS.read_text())['files']
    require(len(sources) == 26, '26 exact Base/Applied source pins required')
    for path, declaration in sources.items():
        require(Path(path).stat().st_size == declaration['bytes'], 'source size changed ' + path)
        verify(path, declaration['sha256'], 'source ' + path)
        artifacts[path] = declaration['sha256']
    for relative, expected in JDK_PINS.items():
        verify(JAVA_HOME / relative, expected, 'JDK ' + relative)
        artifacts[str(JAVA_HOME / relative)] = expected
    inputs = json.loads(ROOT_INPUTS.read_text())
    version = subprocess.run([str(JAVA_HOME / 'bin/java'), '-version'], capture_output=True, text=True, check=True).stderr.strip()
    require(version == inputs['java'], 'JDK version changed')
    bundle = inputs['paper_bundle']
    require(bundle['sha256'] == PIN['paper_bundle'], 'Paper bundle declaration changed')
    verify(bundle['path'], PIN['paper_bundle'], 'Paper bundle')
    artifacts[bundle['path']] = PIN['paper_bundle']
    paths = inputs['paper']
    require(len(paths) == 120, 'expected exact Paper inner plus119 libraries')
    candidates = [path for path, pin in paths.items() if pin == PIN['paper_inner']]
    require(len(candidates) == 1, 'unique pinned Paper inner required')
    inner = candidates[0]
    artifacts.update(paths)
    artifacts[str(Path(__file__).resolve())] = digest(__file__)
    for path, pin in artifacts.items():
        require(digest(path) == pin, 'changed input ' + path)
    reject_shadow(paths)
    return inner, paths, artifacts, version


def main():
    require(len(sys.argv) == 2, 'one fresh output directory required')
    inner, paths, artifacts, version = configuration()
    output = Path(sys.argv[1]).resolve()
    output.mkdir(exist_ok=False)
    classes = output / 'classes'
    classes.mkdir()
    cp = os.pathsep.join([inner, *sorted(path for path in paths if path != inner), str(classes)])
    subprocess.run([str(JAVA_HOME / 'bin/javac'), '-proc:none', '-encoding', 'UTF-8', '-cp', cp, '-d', str(classes), str(HELPER)], check=True)
    with (output / 'paper-material-goldens.log').open('w') as log:
        subprocess.run([str(JAVA_HOME / 'bin/java'), '-Xmx1G', '-cp', cp, 'ferrumc.rootmaterial.ChunkMaterialGoldens', str(FIXTURE), str(output), inner], cwd=output, stdout=log, stderr=subprocess.STDOUT, check=True)
    for path, pin in artifacts.items():
        require(digest(path) == pin, 'changed input after run ' + path)
    rows = [json.loads(row) for row in (output / 'chunks.jsonl').read_text().splitlines()]
    require(len(rows) == 126, 'expected84near+42far')
    require(sum(row['far_end'] for row in rows) == 42, 'expected42farEnd')
    expected = {(name,str(seed),x,z) for name in ('minecraft:amplified','minecraft:caves','minecraft:end','minecraft:floating_islands','minecraft:large_biomes','minecraft:nether','minecraft:overworld') for seed in (0,81985529216486895) for x,z in ((0,0),(1,0),(0,1),(-1,0),(0,-1),(-1,-1))}
    expected |= {('minecraft:end',str(seed),x,z) for seed in (0,1,-1,-9223372036854775808,9223372036854775807,81985529216486895,-81985529205931230) for x,z in ((-32769,0),(32768,0),(0,-32769),(0,32768),(-1875000,-1875000),(1874999,1874999))}
    require({(row['settings'],row['seed'],row['chunk_x'],row['chunk_z']) for row in rows} == expected, 'matrix identities differ')
    require(sum(row['bytes'] for row in rows) == 35389440, 'expected exact full sample bytes')
    require((output / 'material_states.bits').stat().st_size == 35389440, 'unexpected datafile length')
    offset = 0
    identities = set()
    with (output / 'material_states.bits').open('rb') as stream:
        for row in rows:
            require(row['offset'] == offset and row['bytes'] == row['height'] * 256 * 5, 'invalid contiguous range')
            identity = (row['settings'],row['seed'],row['chunk_x'],row['chunk_z'])
            require(identity not in identities, 'duplicate chunk identity')
            identities.add(identity)
            require(hashlib.sha256(stream.read(row['bytes'])).hexdigest() == row['sha256'], 'chunk content hash mismatch')
            offset += row['bytes']
        require(stream.read(1) == b'', 'trailing data')
    class_hashes = {str(path.relative_to(classes)): digest(path) for path in sorted(classes.rglob('*.class'))}
    manifest = {'paper_inner': inner, 'paper_inner_sha256': paths[inner], 'complete_material_chunks':126, 'near_chunks':84, 'far_end_chunks':42, 'samples':7077888, 'bytes':35389440, 'code_origins_checked':25, 'fresh_helper_origin_checked':True, 'helper_last_in_classpath':True, 'jdk_version':version, 'class_hashes':class_hashes, 'output_index':'((y-minY)*16+localX)*16+localZ;big-endian i32 Block.getId(state) followed by u8 fluid-postprocessing flag', 'inputs':artifacts, 'rows_sha256':digest(output/'chunks.jsonl'), 'bits_sha256':digest(output/'material_states.bits'), 'scope':'Actual Paper full material driver and original Aquifer/Ore/default states; no Rust/biome/gameplay acceptance'}
    (output / 'manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
    print(json.dumps({key:manifest[key] for key in ('complete_material_chunks','samples','bits_sha256')}))


if __name__ == '__main__':
    main()
