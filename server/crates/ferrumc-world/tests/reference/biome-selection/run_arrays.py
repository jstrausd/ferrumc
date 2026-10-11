"""Fresh actual createBiomes export; no selector or terrain formula substitute."""
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
    source = Path(__file__).with_name('BiomeArrayOracle.java').resolve()
    root = next(p for p in source.parents if (p / 'AGENTS.md').is_file())
    require(source == root / 'server/crates/ferrumc-world/tests/reference/biome-selection/BiomeArrayOracle.java', 'fresh compiler helper must be the pinned canonical source')
    output = Path(sys.argv[1]).resolve()
    require(not output.exists(), 'output must be fresh')
    pins = json.loads(source.with_name('array-input-pins.json').read_text())
    for relative, pin in pins['repo_inputs'].items():
        require(digest(root / relative) == pin, 'pinned source/input mismatch ' + relative)
    inputs_path = root / '.codex-tmp/gen05_2-root-paper-end/inputs.json'
    inputs = json.loads(inputs_path.read_text())['paper']
    require(len(inputs) == 120, 'exact original inner +119 libraries required')
    inner = root / '.codex-tmp/paper-runtime-1.21.8-60/versions/1.21.8/paper-1.21.8.jar'
    require(inputs[str(inner)] == 'c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507', 'wrong original inner')
    jdk = Path('/home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1')
    artifacts = dict(inputs)
    artifacts.update({str(root / path): pin for path, pin in pins['repo_inputs'].items()})
    artifacts.update({str(jdk / path): pin for path, pin in pins['jdk_inputs'].items()})
    artifacts[str(Path(__file__).resolve())] = digest(__file__)
    artifacts[str(source.with_name('array-input-pins.json'))] = digest(source.with_name('array-input-pins.json'))
    guards = ('JAVA_TOOL_OPTIONS', '_JAVA_OPTIONS', 'JDK_JAVA_OPTIONS', 'JDK_JAVAC_OPTIONS', 'CLASSPATH')
    for name in guards:
        require(not os.environ.get(name), 'Java environment injection ' + name)
    environment = dict(os.environ)
    for name in guards:
        environment.pop(name, None)
    for path, pin in artifacts.items():
        require(digest(path) == pin, 'input hash mismatch ' + path)
    for path in inputs:
        with zipfile.ZipFile(path) as archive:
            require(not any(name.startswith('ferrumc/biomearray/') for name in archive.namelist()), 'helper shadowed by artifact ' + path)
    output.mkdir()
    classes = output / 'classes'
    classes.mkdir()
    cp = os.pathsep.join([str(inner), *sorted(path for path in inputs if path != str(inner)), str(classes)])
    libraries = {
        'com.google.gson.JsonObject': next(path for path in inputs if '/gson/' in path),
        'com.mojang.serialization.Lifecycle': next(path for path in inputs if '/datafixerupper/' in path),
        'com.google.common.collect.ImmutableList': next(path for path in inputs if '/guava/guava/' in path),
        'it.unimi.dsi.fastutil.longs.LongSet': next(path for path in inputs if '/fastutil/' in path),
    }
    libraries_file = output / 'library-origins.json'
    libraries_file.write_text(json.dumps(libraries, indent=2, sort_keys=True) + '\n')
    subprocess.run([str(jdk / 'bin/javac'), '-proc:none', '-encoding', 'UTF-8', '-cp', cp, '-d', str(classes), str(source)], env=environment, check=True)
    with (output / 'arrays.log').open('w') as stream:
        subprocess.run([str(jdk / 'bin/java'), '-Xmx768M', '-cp', cp, 'ferrumc.biomearray.BiomeArrayOracle', str(output), str(inner), str(root / 'server/crates/ferrumc-world/tests/fixtures/biome-selector-original-1.21.8/trees.json'), str(libraries_file)], cwd=output, env=environment, stdout=stream, stderr=subprocess.STDOUT, check=True)
    for path, pin in artifacts.items():
        require(digest(path) == pin, 'input changed after actual export ' + path)
    rows = [json.loads(line) for line in (output / 'arrays.jsonl').read_text().splitlines()]
    require(len(rows) == 156, 'incomplete whole-array corpus')
    counts = {}
    for row in rows:
        source_class = row['source_class'].rsplit('.', 1)[-1]
        counts[source_class] = counts.get(source_class, 0) + 1
        require(row['height'] in (256, 384) and row['min_y'] in (0, -64), 'wrong true full height binding')
        expected_count = row['height'] * 4
        require(len(row['biomes']) == expected_count and len(row['trace']) == expected_count, 'incomplete actual output/trace')
        require(not row['cold'] or row['initial_ordinal'] is None, 'cold source retained history')
        for i, trace in enumerate(row['trace']):
            section, local = divmod(i, 64)
            x, local = divmod(local, 16)
            y, z = divmod(local, 4)
            quart_y = row['min_y'] // 4 + section * 4 + y
            require(trace['quart'] == [row['chunk'][0] * 4 + x, quart_y, row['chunk'][1] * 4 + z], 'original call traversal mismatch')
            storage = ((quart_y - row['min_y'] // 4) * 4 + x) * 4 + z
            require(trace['biome'] == row['biomes'][storage], 'actual trace and actual section getters disagree')
            calls = trace['density_calls']
            if source_class == 'MultiNoiseBiomeSource':
                require(len(calls) == 6 and [call[0] for call in calls] == list(range(6)), 'six-root original sampler trace missing')
                require(all(call[1:4] == [v * 4 for v in trace['quart']] for call in calls), 'wrong original climate sample coordinate')
                require(type(trace['ordinal']) is int, 'missing retained original ordinal')
            elif source_class == 'FixedBiomeSource':
                require(calls == [] and trace['biome'] == 'minecraft:plains', 'fixed source sampled climate or changed binding')
            else:
                require(source_class == 'TheEndBiomeSource' and len(calls) <= 1 and (not calls or calls[0][0] == 3), 'wrong actual End erosion query trace')
        if source_class == 'MultiNoiseBiomeSource':
            require(row['final_ordinal'] == row['trace'][-1]['ordinal'], 'final source history mismatch')
        else:
            require(row['initial_ordinal'] is None and row['final_ordinal'] is None, 'history invented for fixed/End')
    require(counts == {'MultiNoiseBiomeSource': 88, 'TheEndBiomeSource': 64, 'FixedBiomeSource': 4}, 'wrong source corpus')
    end_biomes = {key for row in rows if row['source_class'].endswith('.TheEndBiomeSource') for key in row['biomes']}
    require(end_biomes == {'minecraft:the_end', 'minecraft:end_highlands', 'minecraft:end_midlands', 'minecraft:small_end_islands', 'minecraft:end_barrens'}, 'missing original End branch')
    end_probes = [json.loads(line) for line in (output / 'end-probes.jsonl').read_text().splitlines()]
    require(len(end_probes) == 27, 'missing original End equality probes')
    # A compact acceptance projection retains every section value, original
    # ordered selection ordinal and transformed End f64 call. The complete six-
    # root delegate trace remains separately hashed as the actual export above.
    with (output / 'arrays-compact.jsonl').open('w') as stream:
        for row in rows:
            compact = {key: value for key, value in row.items() if key != 'trace'}
            compact['trace_quarts'] = [trace['quart'] for trace in row['trace']]
            if row['source_class'].endswith('.MultiNoiseBiomeSource'):
                compact['ordinals'] = [trace['ordinal'] for trace in row['trace']]
            if row['source_class'].endswith('.TheEndBiomeSource'):
                compact['end_calls'] = [trace['density_calls'] for trace in row['trace']]
            compact['boundary_density_traces'] = [row['trace'][0]['density_calls'], row['trace'][-1]['density_calls']]
            stream.write(json.dumps(compact, separators=(',', ':')) + '\n')
    receipt = json.loads((output / 'origins.json').read_text())
    require(receipt['original_bootstrap_biomes'] == 65, 'bootstrap biome count changed')
    require(receipt['shared_original_parameter_list_and_rtree_identity_checks'] == 6, 'missing shared original object identity checks')
    manifest = {'inputs': artifacts, 'arrays_sha256': digest(output / 'arrays.jsonl'), 'compact_sha256': digest(output / 'arrays-compact.jsonl'), 'end_probes_sha256': digest(output / 'end-probes.jsonl'), 'origins_sha256': digest(output / 'origins.json'), 'whole_arrays': len(rows), 'source_arrays': counts, 'unwrapped_baselines': sum(row['baseline_verified'] for row in rows), 'origins': receipt, 'scope': 'Actual original createBiomes complete section arrays with original source bindings, empty blending/structure references, traced single delegates and retained same-thread source history; compute evidence only'}
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n')
    print(json.dumps({key: manifest[key] for key in ('arrays_sha256', 'origins_sha256', 'whole_arrays', 'source_arrays', 'unwrapped_baselines')}))


if __name__ == '__main__':
    main()
