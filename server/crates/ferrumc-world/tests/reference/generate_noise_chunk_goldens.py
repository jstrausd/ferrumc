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
    value = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(65536), b''):
            value.update(chunk)
    return value.hexdigest()


def main():
    require(len(sys.argv) == 2, 'one fresh output directory required')
    root = Path(__file__).resolve().parents[5]
    output = Path(sys.argv[1]).resolve()
    output.mkdir(exist_ok=False)
    source = Path(__file__).with_name('ChunkDensityGoldens.java')
    fixture = root / 'docs/fixtures/minecraft-1.21.8-original/worldgen.json'
    inputs_path = root / '.codex-tmp/gen05_2-root-paper-end/inputs.json'
    inputs = json.loads(inputs_path.read_text())
    require(digest(fixture) == 'bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30', 'worldgen source changed')
    for name in ('JAVA_TOOL_OPTIONS', '_JAVA_OPTIONS', 'JDK_JAVA_OPTIONS'):
        require(not os.environ.get(name), 'Java option injection ' + name)
    paths = inputs['paper']
    require(len(paths) == 120, 'expected exact Paper inner plus119 libraries')
    candidates = [path for path, pin in paths.items() if pin == 'c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507']
    require(len(candidates) == 1, 'unique pinned Paper inner required')
    inner = candidates[0]
    artifacts = {**paths, str(source): digest(source), str(fixture): digest(fixture), str(Path(__file__).resolve()): digest(__file__), str(inputs_path): digest(inputs_path)}
    for path, pin in artifacts.items():
        require(digest(path) == pin, 'changed input ' + path)
    for path in paths:
        with zipfile.ZipFile(path) as archive:
            require('ferrumc/rootgoldens/ChunkDensityGoldens.class' not in archive.namelist(), 'shadowed helper ' + path)
    classes = output / 'classes'
    classes.mkdir()
    cp = os.pathsep.join([inner, *sorted(path for path in paths if path != inner), str(classes)])
    java = Path('/home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1/bin')
    subprocess.run([str(java / 'javac'), '-proc:none', '-encoding', 'UTF-8', '-cp', cp, '-d', str(classes), str(source)], check=True)
    with (output / 'paper-goldens.log').open('w') as log:
        subprocess.run([str(java / 'java'), '-Xmx1G', '-cp', cp, 'ferrumc.rootgoldens.ChunkDensityGoldens', str(fixture), str(output), inner], cwd=output, stdout=log, stderr=subprocess.STDOUT, check=True)
    for path, pin in artifacts.items():
        require(digest(path) == pin, 'changed input after run ' + path)
    rows = [json.loads(row) for row in (output / 'chunks.jsonl').read_text().splitlines()]
    require(len(rows) == 126, 'expected84near+42far')
    require(sum(row['far_end'] for row in rows) == 42, 'expected42farEnd')
    require(sum(row['bytes'] for row in rows) == 56623104, 'expected exact full sample bytes')
    require((output / 'material_density.bits').stat().st_size == 56623104, 'unexpected datafile length')
    offset = 0
    identities = set()
    with (output / 'material_density.bits').open('rb') as stream:
        for row in rows:
            require(row['offset'] == offset and row['bytes'] == row['height'] * 256 * 8, 'invalid contiguous range')
            identity = (row['settings'],row['seed'],row['chunk_x'],row['chunk_z'])
            require(identity not in identities, 'duplicate chunk identity')
            identities.add(identity)
            require(hashlib.sha256(stream.read(row['bytes'])).hexdigest() == row['sha256'], 'chunk content hash mismatch')
            offset += row['bytes']
        require(stream.read(1) == b'', 'trailing data')
    manifest = {'paper_inner': inner, 'paper_inner_sha256': paths[inner], 'complete_density_chunks':126, 'near_chunks':84, 'far_end_chunks':42, 'samples':7077888, 'bytes':56623104, 'code_origins_checked':18, 'fresh_helper_origin_checked':True, 'output_index':'((y-minY)*16+localX)*16+localZ;big-endian raw f64 bits', 'inputs':artifacts, 'rows_sha256':digest(output/'chunks.jsonl'), 'bits_sha256':digest(output/'material_density.bits'), 'scope':'Actual Paper constructor material-density driver only; no Aquifer/block/biome/gameplay acceptance'}
    (output / 'manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
    print(json.dumps({key:manifest[key] for key in ('complete_density_chunks','samples','bits_sha256')}))


if __name__ == '__main__':
    main()
