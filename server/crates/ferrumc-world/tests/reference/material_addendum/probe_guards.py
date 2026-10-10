"""Controlled refusals against the frozen exporter; no reference input mutations."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    require(len(sys.argv) == 3, 'VALID_EXPORT FRESH_TEST_DIR required')
    work = Path(__file__).resolve().parent
    export = Path(sys.argv[1]).resolve()
    output = Path(sys.argv[2]).resolve()
    require(output.is_relative_to(Path(__file__).resolve().parents[6] / '.codex-tmp'), 'output outside repository scratch')
    output.mkdir(exist_ok=False)
    spec = importlib.util.spec_from_file_location('branch_exporter', work / 'run_addendum.py')
    runner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runner)
    saved_argv = sys.argv.copy()
    records = []
    for name in runner.ENV:
        require(not os.environ.get(name), 'unexpected preexisting Java injection')
        os.environ[name] = '-Doracle.unexpected=true'
        sys.argv = [str(work / 'run_addendum.py'), str(output / name)]
        try:
            runner.main()
        except ValueError as error:
            require(str(error) == 'refuse Java option injection ' + name,
                    'wrong refusal ' + str(error))
            records.append({'probe': name, 'refusal': str(error), 'scope': 'real environment injection'})
        else:
            raise ValueError('option injection accepted')
        finally:
            del os.environ[name]

    original_digest = runner.digest
    worldgen = Path(__file__).resolve().parents[6] / 'docs/fixtures/minecraft-1.21.8-original/worldgen.json'
    manifest=json.loads((export/'manifest.json').read_text())
    source_pins=json.loads((work.parent/'material-source-pins.json').read_text())
    aquifer=next(Path(path) for path in source_pins['files'] if path.endswith('/src/minecraft/java/net/minecraft/world/level/levelgen/Aquifer.java'))
    faults=[('worldgen_hash',worldgen,'worldgen pin drift'),
            ('paper_inner_hash',Path(manifest['paper_inner']),'changed input '+manifest['paper_inner']),
            ('approved_source_hash',aquifer,'changed input '+str(aquifer))]
    for label,target,refusal in faults:
        def changed_digest(path):
            if Path(path).resolve() == target.resolve():return '0'*64
            return original_digest(path)
        runner.digest = changed_digest
        sys.argv = [str(work / 'run_addendum.py'), str(output / label)]
        try:
            runner.main()
        except ValueError as error:
            require(str(error)==refusal,'wrong hash refusal '+str(error))
            records.append({'probe':label,'refusal':str(error),
                            'scope':'controlled digest fault injection; real pinned reference unchanged'})
        else:
            raise ValueError('hash drift accepted')
        finally:
            runner.digest=original_digest
            sys.argv=saved_argv

    invocation = json.loads((export / 'invocation.json').read_text())
    command = invocation['run_command'].copy()
    # Existing helper directory is deliberately declared as the original JAR.
    # The actual loaded BlockPos class must refuse this before bootstrap.
    command[-1] = str(export / 'classes')
    command[-2] = str(output)
    process = subprocess.run(command, env=invocation['explicit_java_environment'],
                             cwd=output, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                             text=True, check=False)
    (output / 'wrong-origin.log').write_text(process.stdout)
    require(process.returncode != 0, 'wrong original origin accepted')
    require('wrong original origin net.minecraft.core.BlockPos:' in process.stdout,
            'wrong original origin not refused before bootstrap')
    records.append({'probe': 'original_class_codesource', 'refusal': 'wrong original origin net.minecraft.core.BlockPos',
                    'exit_code': process.returncode, 'scope': 'actual Java process with wrong declared original origin'})
    command=invocation['run_command'].copy()
    command[-2]=str(output)
    (output/'classes').mkdir()
    process=subprocess.run(command,env=invocation['explicit_java_environment'],cwd=output,
                           stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,check=False)
    (output/'wrong-helper-origin.log').write_text(process.stdout)
    require(process.returncode!=0,'wrong helper origin accepted')
    require('wrong fresh helper origin class ferrumc.branchaddendum.MaterialBranchAddendum' in process.stdout,'wrong helper origin not refused before bootstrap')
    records.append({'probe':'fresh_helper_codesource','refusal':'wrong fresh helper origin class ferrumc.branchaddendum.MaterialBranchAddendum',
                    'exit_code':process.returncode,'scope':'actual Java process with correct original JAR and wrong declared fresh helper directory'})
    require(not (output / 'addendum.jsonl').exists(), 'negative run published branch output')
    require(not (output / 'origins.json').exists(), 'negative run completed original origin checks')
    for name in (*runner.ENV, 'worldgen_hash','paper_inner_hash','approved_source_hash'):
        require(not (output / name / 'classes').exists(), 'negative exporter reached compilation')
    (output / 'guards.json').write_text(json.dumps(records, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'negative_guards': len(records), 'all_refused': True}))


if __name__ == '__main__':
    main()
