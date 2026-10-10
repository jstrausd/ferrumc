#!/usr/bin/env python3
"""Genuine isolated Paper boot; no recreated packet/item serializer."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import zipfile

PAPER_OUTER_SHA = '8de7c52c3b02403503d16fac58003f1efef7dd7a0256786843927fa92ee57f1e'
JAVA_ROOT = Path('/home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1/bin')


def digest(path):
    return hashlib.file_digest(Path(path).open('rb'), 'sha256').hexdigest()


def verify(pins):
    for path, sha in pins.items():
        if digest(path) != sha:
            raise RuntimeError(f'input hash mismatch: {path}')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--java-bin', type=Path, default=JAVA_ROOT)
    parser.add_argument('--runtime', type=Path)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--artifact-pins', type=Path, default=Path(__file__).with_name('artifact-pins.json'))
    args = parser.parse_args()
    root = args.root.resolve()
    out = args.output.resolve()
    if out.exists():
        raise RuntimeError('use a fresh output directory')
    out.mkdir(parents=True)
    runtime = (args.runtime or root / '.codex-tmp/paper-runtime-1.21.8-60').resolve()
    artifact_pins = json.loads(args.artifact_pins.read_text())
    pins = {str(runtime / path): sha for path, sha in artifact_pins.items()}
    java_bin = args.java_bin.resolve()
    outer = runtime / 'paper-1.21.8-60.jar'
    inner = runtime / 'versions/1.21.8/paper-1.21.8.jar'
    if len(pins) != 121 or pins.get(str(outer)) != PAPER_OUTER_SHA:
        raise RuntimeError('invalid original artifact set')
    verify(pins)
    source_root = (args.source_root or root / '.codex-tmp/paper-1.21.8-reference').resolve()
    source_file = Path(__file__).with_name('source-pins.json')
    source_records = json.loads(source_file.read_text())
    source_pins = {str(source_root / row['path']): row['sha256'] for row in source_records['rows']}
    verify(source_pins)
    classpath = os.pathsep.join(sorted(p for p in pins if p != str(outer)))
    helper = Path(__file__).resolve().with_name('ItemEntityWirePlugin.java')
    classes = out / 'classes'
    classes.mkdir()
    compile_command = [str(java_bin / 'javac'), '-proc:none', '-encoding', 'UTF-8', '-cp', classpath, '-d', str(classes), str(helper)]
    result = subprocess.run(compile_command, text=True, capture_output=True)
    (out / 'compile.log').write_text(result.stdout + result.stderr)
    result.check_returncode()
    classfile = classes / 'net04/ItemEntityWirePlugin.class'
    plugin = out / 'wire-reference.jar'
    descriptor = 'name: Net04WireReference\nversion: 1\nmain: net04.ItemEntityWirePlugin\napi-version: 1.21.8\nload: POSTWORLD\n'
    manifest = 'Manifest-Version: 1.0\npaperweight-mappings-namespace: mojang\n\n'
    with zipfile.ZipFile(plugin, 'w') as jar:
        # Stable timestamps make helper Jar hashes independently reproducible.
        for name, data in [('META-INF/MANIFEST.MF', manifest.encode()), ('plugin.yml', descriptor.encode()),
                           ('net04/ItemEntityWirePlugin.class', classfile.read_bytes())]:
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            jar.writestr(info, data)
    boot = out / 'server'
    boot.mkdir()
    (boot / 'plugins').mkdir()
    shutil.copyfile(plugin, boot / 'plugins/wire-reference.jar')
    for name in ['libraries', 'versions', 'cache']:
        (boot / name).symlink_to(runtime / name, target_is_directory=True)
    (boot / 'eula.txt').write_text('eula=true\n')
    (boot / 'server.properties').write_text('''server-ip=127.0.0.1
server-port=0
online-mode=false
level-name=reference_world
level-seed=0
level-type=minecraft:flat
generator-settings={"layers":[{"block":"minecraft:bedrock","height":1},{"block":"minecraft:dirt","height":2},{"block":"minecraft:grass_block","height":1}],"biome":"minecraft:plains","features":false,"lakes":false}
generate-structures=false
allow-nether=false
view-distance=2
simulation-distance=2
max-players=1
network-compression-threshold=-1
enable-query=false
enable-rcon=false
spawn-protection=0
sync-chunk-writes=true
motd=NET04 isolated reference export
''')
    (boot / 'bukkit.yml').write_text('settings:\n  allow-end: false\n  connection-throttle: 4000\n')
    (boot / 'config').mkdir()
    (boot / 'config/paper-global.yml').write_text('''_version: 30
anticheat:
  obfuscation:
    items:
      enable-item-obfuscation: false
''')
    pre_configs = {str(p.relative_to(boot)): digest(p) for p in [boot / 'eula.txt', boot / 'server.properties', boot / 'bukkit.yml', boot / 'config/paper-global.yml']}
    command = [str(java_bin / 'java'), '-Xms256M', '-Xmx1024M', '-Dnet04.paper.inner=' + str(inner),
               '-Dnet04.helper.class.sha256=' + digest(classfile),
               '-jar', str(outer), '--nogui']
    environment = dict(os.environ)
    environment['PATH'] = str(java_bin) + os.pathsep + environment.get('PATH', '')
    # Keep connector cache writes in the fresh scratch.
    environment['XDG_CACHE_HOME'] = str(out / 'cache')
    with (out / 'server.log').open('w') as log:
        process = subprocess.Popen(command, cwd=boot, env=environment, stdin=subprocess.PIPE, stdout=log, stderr=subprocess.STDOUT)
        try:
            status = process.wait(timeout=300)
        except subprocess.TimeoutExpired:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            raise RuntimeError('reference boot timed out; process stopped')
    verify(pins)
    verify(source_pins)
    if status != 0 or not (boot / 'SUCCESS').is_file() or (boot / 'FAILED').exists():
        raise RuntimeError(f'actual Paper export failed (exit {status}); see server.log')
    observations = json.loads((boot / 'paper-observations.json').read_text())
    if observations['rows'] != 30:
        raise RuntimeError(f'unexpected row count: {observations["rows"]}')
    if observations['item_entity_metadata_index'] != 8 or observations['item_stack_serializer_id'] != 7:
        raise RuntimeError('actual ItemEntity metadata registration changed')
    shutil.copyfile(boot / 'paper.tsv', out / 'paper.tsv')
    shutil.copyfile(boot / 'paper-observations.json', out / 'paper-observations.json')
    configs = {str(p.relative_to(boot)): {'sha256': digest(p), 'bytes': p.stat().st_size}
               for p in sorted(boot.glob('*.yml')) + sorted((boot / 'config').glob('*.yml')) + [boot / 'server.properties', boot / 'eula.txt']}
    provenance = {'status': 'passed', 'scope': 'actual original packet codecs during genuine isolated server boot; no gameplay acceptance',
                  'paper_commit': '29c8822d90899c89d2689338e81a98f690bcba12', 'protocol': 772, 'data_version': 4440,
                  'artifacts': pins, 'helper_source_sha256': digest(helper), 'helper_class_sha256': digest(classfile),
                  'helper_jar_sha256': digest(plugin), 'runner_sha256': digest(__file__), 'artifact_pins_sha256': digest(args.artifact_pins),
                  'source_pins_sha256': digest(source_file), 'sources': source_pins, 'compile_command': compile_command,
                  'command': command, 'exit_code': status, 'pre_config_sha256': pre_configs, 'post_configs': configs,
                  'java_version': subprocess.run([str(java_bin / 'java'), '-version'], capture_output=True, text=True, check=True).stderr,
                  'outputs': {p.name: digest(p) for p in [out / 'paper.tsv', out / 'paper-observations.json']}}
    (out / 'provenance.json').write_text(json.dumps(provenance, sort_keys=True, indent=2) + '\n')
    print(json.dumps({'rows': observations['rows'], 'packet_sha256': digest(out / 'paper.tsv'), 'output': str(out)}, sort_keys=True))


if __name__ == '__main__':
    main()
