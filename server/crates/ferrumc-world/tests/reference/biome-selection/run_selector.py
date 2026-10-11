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
    source = Path(__file__).with_name('BiomeSelectorOracle.java')
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
            require('ferrumc/rootselector/BiomeSelectorOracle.class' not in archive.namelist(), 'helper shadowed ' + path)
    output.mkdir()
    classes = output / 'classes'
    classes.mkdir()
    cp = os.pathsep.join([inner, *sorted(path for path in inputs if path != inner), str(classes)])
    subprocess.run([str(jdk/'bin/javac'), '-proc:none', '-encoding', 'UTF-8', '-cp', cp, '-d', str(classes), str(source)], check=True)
    with (output / 'selector.log').open('w') as stream:
        subprocess.run([str(jdk/'bin/java'), '-Xmx512M', '-cp', cp, 'ferrumc.rootselector.BiomeSelectorOracle', str(output), inner], cwd=output, stdout=stream, stderr=subprocess.STDOUT, check=True)
    for path, pin in artifacts.items():
        require(digest(path) == pin, 'input changed after export ' + path)
    queries = [json.loads(line) for line in (output/'queries.jsonl').read_text().splitlines()]
    trees = json.loads((output/'trees.json').read_text())
    require(set(trees) == {'minecraft:overworld', 'minecraft:nether'}, 'wrong presets')
    summary = {}
    for preset, count in (('minecraft:overworld',7593),('minecraft:nether',5)):
        nodes = trees[preset]
        leaves = [node['leaf'] for node in nodes if 'leaf' in node]
        require(sorted(leaves) == list(range(count)), 'missing or duplicate original leaves')
        for index,node in enumerate(nodes):
            require(node['index'] == index and len(node['axes']) == 7, 'wrong original node shape')
            require(all(len(axis) == 2 and axis[0] <= axis[1] for axis in node['axes']), 'invalid original ranges')
            if 'children' in node:
                require(1 <= len(node['children']) <= 6 and all(index < child < len(nodes) for child in node['children']), 'invalid original child order')
        rows = [row for row in queries if row['preset'] == preset]
        expected_per_group = min(32,count)*25+84
        for group in ('cold','forward','reverse'):
            grouped = [row for row in rows if row['group'] == group]
            require(len(grouped) == expected_per_group and [row['sequence'] for row in grouped] == list(range(expected_per_group)), 'wrong query group/order')
        require(all(len(row['target']) == 6 and all(type(value) is int for value in row['target']) for row in rows), 'wrong query input')
        summary[preset] = {'nodes':len(nodes),'leaves':count,'queries':len(rows),'index_vs_brute_differences':sum(row.get('indexed') != row['brute_force'] for row in rows),'original_errors':sum('indexed_error' in row for row in rows)}
    ties = [row for row in queries if row['preset'] == 'declared-two-point-tie']
    require(len(ties) == 8, 'missing declared tie probes')
    require([row['indexed'] for row in ties[:4]] == [0,0,1,1], 'wrong actual forward tie history')
    require([row['indexed'] for row in ties[4:]] == [1,1,0,0], 'wrong actual reverse tie history')
    manifest = {'inputs':artifacts, 'queries_sha256':digest(output/'queries.jsonl'), 'trees_sha256':digest(output/'trees.json'), 'original_class_origins_checked':20, 'fresh_helper_origin_checked':True, 'presets':summary, 'declared_tie_probes':8, 'scope':'Actual Paper builtin RTree structure and selectors, declared target probes, cold/forward/reverse and tie history only; no Rust selector, biome/surface or gameplay acceptance'}
    (output/'manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
    print(json.dumps({'presets':summary,'queries_sha256':manifest['queries_sha256'],'trees_sha256':manifest['trees_sha256']}))


if __name__ == '__main__':
    main()
