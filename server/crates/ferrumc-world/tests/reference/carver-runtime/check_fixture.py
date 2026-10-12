"""Exact additive fixture provenance, memberships and discriminating classes."""
from pathlib import Path
import csv,hashlib,json,sys
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parents[6]
FIX=ROOT/'server/crates/ferrumc-world/tests/fixtures/carver-runtime-original-1.21.8'
def h(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def require(c,m):
 if not c:raise ValueError(m)
pins=json.loads((FIX/'runtime-inputs.json').read_text())['files']
require({p.name for p in FIX.iterdir() if p.is_file()}==set(pins)|{'runtime-inputs.json'},'fixture exact membership')
for name,expected in pins.items():require(h(FIX/name)==expected,'fixture bytes '+name)
old=ROOT/'.codex-tmp/gen05_11/fixture/carver-original-1.21.8'
original={p.name for p in old.iterdir() if p.is_file()}
require(len(original)==11,'original eleven-file membership')
for name in original:require((FIX/name).read_bytes()==(old/name).read_bytes(),'historical input '+name)
# Additions must be declared members of their pinned original freezes.
provenance = {}
authorities = [
 ('gen05_11-r2-independent-review', 'freeze-manifest.json', '2c7a2527b3173f933342b7db70fe0f0c36adcfe6fa72cc2b020c6d32e7da4f26'),
 ('gen05_11', 'delivery/freeze-manifest.json', '6451131f336c011486edcfc91d1a43a38274f4e5e390b48e3b5ba72f88b29863'),
]
for name in sorted(set(pins)-original):
 matches=[]
 for area,manifest,pin in authorities:
  base=ROOT/'.codex-tmp'/area;freeze=base/manifest
  require(h(freeze)==pin,'authority freeze '+area)
  members=json.loads(freeze.read_text())['files']
  for path,digest in members.items():
   if digest==pins[name] and path.endswith('.tsv'):
    require(h(base/path)==digest,'frozen original '+path)
    matches.append({'authority':str(freeze.relative_to(ROOT)), 'authority_sha256':pin, 'path':str((base/path).relative_to(ROOT)), 'sha256':digest})
 require(bool(matches),'declared frozen original provenance '+name)
 provenance[name]=matches
def rows(name):return list(csv.DictReader((FIX/name).open(),delimiter='\t'))
groups=rows('groups.tsv');vanilla=rows('vanilla.tsv')
require(len(groups)==73 and len({r['group'] for r in groups})==73,'group membership')
classes={r['group']:r['class'] for r in groups if r['class']!='full'}
require(classes=={'outside/normal/0/+x':'refused:CentreOutsideDomain','policy/single_biome_surface/0':'refused:InconsistentResolver(Source)'},'classes')
require(sum(r['equal_to_paper']=='0' for r in vanilla)==25,'discriminating Vanilla groups')
require({r['group'] for r in vanilla}=={r['group'] for r in groups},'Vanilla exact groups')
report={'fixture_files':len(pins)+1,'original_unchanged':len(original),'additions':sorted(set(pins)-original),'classes':classes,'full_groups':71,'vanilla_discriminating_groups':25,'provenance':provenance,'all_checks_passed':True}
label=sys.argv[1];out=ROOT/'.codex-tmp/gen05_11-runtime'/label
require(not out.exists(),'fresh evidence required');out.write_text(json.dumps(report,indent=2,sort_keys=True)+'\n');print(json.dumps(report))
