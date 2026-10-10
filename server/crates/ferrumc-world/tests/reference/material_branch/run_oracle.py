"""Fresh actual-JAR exports, with explicit environment and pre/post pin checks."""
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import zipfile

PIN = {
    'helper': '9b35ff4e2b3eb1160d3c30cfce905256a28fe2965eea52c65ea0b6b58ec47b22',
    'cases': 'e5e6862cb8e386932561cc7bb0cdb0cc3dbbed7a6cb44c2c798e2728a0289d37',
    'builder': '06994084fcf4252300f317fc898b568eebaafce401337fd0d76ece38907ffce3',
    'root_inputs': 'd7eff61da418e8a8883f69dd2b079cc5c5b1a91612f16745f3c07ec90ff58841',
    'source_pins': '6619af5dc820d889f1bc13f650eb070c699b0d8e76231e67d68c7a2b9be447bf',
}
JDK_PINS = {
    'java': '2a207f5e7d075afa01d97f8048389a64432a44c4a5af0f5e77d6e286ec5f401d',
    'javac': '55859b80e7a9c4c4736be19ad3addeb35112ca6d17a30c4e0e116afc0a499bdb',
    '../lib/modules': '70c5465dabf8e1bec2860631ea3ffd342f0ec23a5e2efd3a389d2abe32448f98',
}
ENV = ('JAVA_TOOL_OPTIONS','_JAVA_OPTIONS','JDK_JAVA_OPTIONS','JDK_JAVAC_OPTIONS','CLASSPATH')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    value=hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(65536),b''):
            value.update(chunk)
    return value.hexdigest()


def main():
    require(len(sys.argv)==2,'one fresh output path is required')
    work=Path(__file__).resolve().parent
    root=Path(__file__).resolve().parents[6]
    output=Path(sys.argv[1]).resolve()
    require(output.is_relative_to(root/'.codex-tmp'),'output must stay in repository scratch')
    output.mkdir(exist_ok=False)
    source=work/'MaterialBranchOracle.java'
    cases=work/'cases.jsonl'
    worldgen=root/'docs/fixtures/minecraft-1.21.8-original/worldgen.json'
    inputs_path=root/'.codex-tmp/gen05_2-root-paper-end/inputs.json'
    source_pins_path=work.parent/'material-source-pins.json'
    for path,label in [(source,'helper'),(cases,'cases'),(work/'make_cases.py','builder'),
                       (inputs_path,'root_inputs'),(source_pins_path,'source_pins')]:
        require(digest(path)==PIN[label],label+' pin drift')
    inputs=json.loads(inputs_path.read_text())
    source_pins=json.loads(source_pins_path.read_text())
    paths=inputs['paper']
    require(len(paths)==120,'expected exact Paper inner plus119 libraries')
    inner=[path for path,pin in paths.items() if pin=='c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507']
    require(len(inner)==1,'unique pinned Paper inner required')
    inner=inner[0]
    require(digest(worldgen)=='bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30','worldgen pin drift')
    require(len(source_pins['files'])==26,'expected26 approved source pins')
    artifacts={**paths,**{path:pin['sha256'] for path,pin in source_pins['files'].items()},
               **{str(path):digest(path) for path in [source,cases,worldgen,inputs_path,source_pins_path,Path(__file__).resolve(),work/'make_cases.py']}}
    for name in ENV:
        require(not os.environ.get(name),'refuse Java option injection '+name)
    for path,pin in artifacts.items():
        require(digest(path)==pin,'changed input '+path)
    for path in paths:
        with zipfile.ZipFile(path) as archive:
            require(not any(name.startswith('ferrumc/branchoracle/') for name in archive.namelist()),'shadowed helper '+path)
    rows=[json.loads(line) for line in cases.read_text().splitlines()]
    require(len(rows)==499,'expected499 declared cases')
    expected_counts={'pressure':69,'surface_level':151,'random_surface':70,'fluid_type':66,'ore':107,'disabled_substance':36}
    require(dict(Counter(row['kind'] for row in rows))==expected_counts,'unexpected endpoint matrix')
    require(len({(row['kind'],row['case']) for row in rows})==len(rows),'duplicate input row')
    require(all(len(row['inputs'].get('float_bits',[]))<=3 for row in rows),'oversized declared RNG array')
    classes=output/'classes';classes.mkdir()
    temporary=output/'tmp';temporary.mkdir()
    cp=os.pathsep.join([inner,*sorted(path for path in paths if path!=inner),str(classes)])
    java=Path('/home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1/bin')
    for relative,pin in JDK_PINS.items():
        path=(java/relative).resolve()
        require(digest(path)==pin,'JDK pin drift '+relative)
        artifacts[str(path)]=pin
    version=subprocess.run([str(java/'java'),'-version'],capture_output=True,text=True,check=True).stderr.strip()
    require(version==inputs['java'],'JDK version drift')
    # Deliberately do not inherit the global process environment into Java.
    env={'PATH':str(java)+':/usr/bin:/bin','LANG':'C.UTF-8','LC_ALL':'C.UTF-8','TMPDIR':str(temporary)}
    compile_command=[str(java/'javac'),'-proc:none','-encoding','UTF-8','-cp',cp,'-d',str(classes),str(source)]
    with (output/'javac.log').open('w') as log:
        subprocess.run(compile_command,check=True,env=env,stdout=log,stderr=subprocess.STDOUT,cwd=output)
    run_command=[str(java/'java'),'-Xmx1G','-Djava.io.tmpdir='+str(temporary),'-cp',cp,
                 'ferrumc.branchoracle.MaterialBranchOracle',str(worldgen),str(cases),str(output),inner]
    with (output/'oracle.log').open('w') as log:
        subprocess.run(run_command,check=True,env=env,stdout=log,stderr=subprocess.STDOUT,cwd=output)
    for path,pin in artifacts.items():
        require(digest(path)==pin,'changed input after run '+path)
    actual=[json.loads(line) for line in (output/'branches.jsonl').read_text().splitlines()]
    require(len(actual)==len(rows),'missing actual rows')
    for given,observed in zip(rows,actual,strict=True):
        require(all(observed[key]==given[key] for key in ('kind','case','inputs')),'input mutation or row reordering')
        require(all(key in observed for key in ('result','events','requests','rng_events')),'incomplete trace shape')
        require([event for event in observed['events'] if event['kind']=='density']==observed['requests'],'density chronology differs')
        require([event for event in observed['events'] if event['kind']!='density']==observed['rng_events'],'RNG chronology differs')
    origins=json.loads((output/'origins.json').read_text())
    require(len(origins)==29,'expected29 actual original classes')
    require(all(Path(entry['origin']).resolve()==Path(inner).resolve() for entry in origins),'wrong original class origin')
    manifest={'paper_inner':inner,'paper_inner_sha256':paths[inner],
              'rows':len(actual),'groups':expected_counts,'original_class_origins':len(origins),
              'fresh_helper_origins':5,
              'jdk_version':version,
              'class_hashes':{str(path.relative_to(classes)):digest(path) for path in sorted(classes.rglob('*.class'))},
              'java_environment_policy':'only PATH/LANG/LC_ALL/TMPDIR supplied; no inherited global environment',
              'inputs':artifacts,
              'branches_sha256':digest(output/'branches.jsonl'),
              'origins_sha256':digest(output/'origins.json'),
              'declared_leaves':'constant f64 inputs, not an expected-value implementation',
              'declared_rng':'at most3 raw f32 values; actual OreVeinifier alone chooses draws and branches',
              'actual_rng_cases':sum('actual_seed' in row['inputs'] for row in actual),
              'scope':'actual Paper primitive endpoint branch oracle; no Rust/material/gameplay acceptance'}
    (output/'manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True)+'\n')
    # Output-specific invocation paths belong in provenance, keeping the input
    # and result manifest byte-identical across the normal and -O exporters.
    (output/'invocation.json').write_text(json.dumps({'explicit_java_environment':env,
        'compile_command':compile_command,'run_command':run_command},indent=2,sort_keys=True)+'\n')
    print(json.dumps({key:manifest[key] for key in ('rows','groups','branches_sha256')}))


if __name__=='__main__':
    main()
