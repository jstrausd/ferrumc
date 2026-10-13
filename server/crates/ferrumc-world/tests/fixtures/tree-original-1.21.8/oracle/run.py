"""Reproduce actual Paper kernels and explicit tree-subset original-class probes."""
from pathlib import Path
import json,hashlib,subprocess,os,zipfile,tempfile,sys
area=Path(__file__).resolve().parent.parent
root=Path(os.environ.get('FERRUMC_REFERENCE_ROOT','/home/jstrauss/Schreibtisch/mchosting/ferrumc'))
old=root/'.codex-tmp/gen06_1/oracle'
pins=json.loads((old/'input-pins.json').read_text());jdk=Path(pins['jdk']);jars=pins['runtime_inputs']['paper'];inner=next(p for p in jars if '/versions/1.21.8/' in p)
def require(condition,message):
 if not condition:raise RuntimeError(message)
for k in ['JAVA_TOOL_OPTIONS','_JAVA_OPTIONS','JDK_JAVA_OPTIONS','JDK_JAVAC_OPTIONS','CLASSPATH']:require(not os.environ.get(k),k)
inputs={**jars,**pins['jdk_inventory']};inputs[str(old/'symbols.properties')]=pins['inventory'][str(old/'symbols.properties')]
def verify():
 for p,h in inputs.items():require(hashlib.sha256(Path(p).read_bytes()).hexdigest()==h,p)
verify()
for p in jars:
 with zipfile.ZipFile(p) as z:require(not any(n.startswith('ferrumc/treeoracle/') for n in z.namelist()),'helper namespace in pinned input')
with tempfile.TemporaryDirectory(prefix='ferrumc-tree-original-') as temporary:
 work=Path(temporary);classes=work/'classes';classes.mkdir();cp=os.pathsep.join([inner,*sorted(p for p in jars if p!=inner)])
 subprocess.run([str(jdk/'bin/javac'),'-proc:none','-cp',cp,'-d',str(classes),str(area/'oracle/TreeOracle.java')],check=True)
 (area/'original.tsv.done').unlink(missing_ok=True)
 with (work/'oracle.log').open('w') as log:subprocess.run([str(jdk/'bin/java'),'-Xmx1G','-Dmax.bg.threads=1','-cp',cp+os.pathsep+str(classes),'ferrumc.treeoracle.TreeOracle',inner,str(classes),str(old/'symbols.properties'),str(area/'original.tsv')],check=True,stdout=log,stderr=subprocess.STDOUT,cwd=work)
 require((area/'original.tsv.done').read_text()=='TREE_DONE\n','missing actual completion marker')
 (area/'original.tsv.done').unlink()
with zipfile.ZipFile(inner) as z:
 for name,typ in [('oak','configured_feature'),('birch','configured_feature'),('trees_birch','placed_feature'),('trees_birch_and_oak_leaf_litter','placed_feature'),('trees_plains','placed_feature')]:
  (area.parent.parent.parent/'src/tree_resources'/f'{name}.json').write_bytes(z.read(f'data/minecraft/worldgen/{typ}/{name}.json'))
subprocess.run([sys.executable,str(area/'oracle/import_facts.py')],check=True)
verify()
print('TREE_DONE')
