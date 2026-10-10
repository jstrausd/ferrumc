"""Bounded declared inputs only, with no expected selection/cache/surface formula."""
from copy import deepcopy
from pathlib import Path
import json
import math
import struct

def bits(value):return struct.pack('>d',value).hex()
base={
 'x':1,'y':25,'z':2,'context':'NoiseChunk','actual_seed':'0','endpoint':'enabled_substance',
 'leaves':{key:bits(value) for key,value in {'barrier':0.5,'floodedness':1.0,'spread':0.0,'lava':0.0,'erosion':0.0,'depth':0.0,'toggle':0.8,'ridged':-1.0,'gap':0.0}.items()},
 'preliminary':{'default_f64_bits':bits(0.5),'overrides':[]},
 'picker':{'default':{'height':63,'fluid':'water'},'overrides':[]},
 'calls':[{'substance_f64_bits':bits(value)} for value in (-1.0,0.0,-0.0,math.nextafter(0.0,math.inf),0.0)]}
rows=[]
def add(kind,name,policy='finite_parity',**changes):
 inputs=deepcopy(base);inputs.update(changes)
 rows.append({'kind':kind,'case':name,'policy':policy,'inputs':inputs})
for seed in ('0','-1','1311768467463790320'):
 for context in ('NoiseChunk','SinglePoint'):
  for x,y,z in ((1,25,2),(3,62,15),(0,-54,0)):
   add('enabled_substance',f'seed{seed}_{context}_{x}_{y}_{z}',actual_seed=seed,context=context,x=x,y=y,z=z)
for fluid in ('water','lava','air'):
 for height in (24,25,26):
  add('enabled_substance',f'global_{fluid}_{height}',picker={'default':{'height':height,'fluid':fluid},'overrides':[]},calls=[{'substance_f64_bits':bits(value)} for value in (0.0,-0.0)])
centers=[{'grid':[x,y,z],'point':[x*16,y*12,z*16]} for x in (0,1) for y in (1,2,3) for z in (0,1)]
# Four declared equidistant points about8/24/8; original computeSubstance
# alone determines their selected order. No expected ranking is computed.
for mode,target in [('same',None),('nearest_diff',(16,24,16)),('second_diff',(16,24,0)),('third_diff',(0,24,16)),('fourth_diff',(0,24,0)),('nearest_lava',(16,24,16))]:
 picker={'default':{'height':40,'fluid':'water'},'overrides':[]}
 if target:
  picker['overrides'].append({'x':target[0],'y':target[1],'z':target[2],'status':{'height':41,'fluid':'lava' if mode=='nearest_lava' else 'water'}})
 add('enabled_ties',mode,x=8,y=24,z=8,context='SinglePoint',centers=centers,picker=picker,calls=[{'substance_f64_bits':bits(value)} for value in (-100.0,0.0,-0.0,math.nextafter(0.0,math.inf),-100.0)])
for name in ('full13_air_surface','full13_fluid_present','central_above_surface','central_surface_submerged','negative3_neighbor','max_surface_wrap'):
 changes={'endpoint':'fluid_scan','context':'SinglePoint','calls':[{},{}]}
 if name=='full13_fluid_present':changes['picker']={'default':{'height':400,'fluid':'water'},'overrides':[]}
 if name in ('central_above_surface','central_surface_submerged'):
  changes['preliminary']={'default_f64_bits':bits(0.0),'overrides':[{'x':0,'y':32,'z':0,'value_f64_bits':bits(0.5)}]}
  changes['y']=80 if name=='central_above_surface' else 40
 if name=='negative3_neighbor':
  changes['preliminary']={'default_f64_bits':bits(0.5),'overrides':[{'x':-48,'y':y,'z':0,'value_f64_bits':bits(0.0)} for y in range(320,24,-8)]}
 if name=='max_surface_wrap':changes['preliminary']={'default_f64_bits':bits(0.0),'overrides':[]}
 add('fluid_scan',name,**changes)
for name in ('full13','max_surface_wrap','surface32'):
 preliminary={'default_f64_bits':bits(0.5),'overrides':[]}
 if name=='max_surface_wrap':preliminary['default_f64_bits']=bits(0.0)
 if name=='surface32':preliminary={'default_f64_bits':bits(0.0),'overrides':[{'x':0,'y':32,'z':0,'value_f64_bits':bits(0.5)}]}
 add('fluid_cache',name,endpoint='fluid_cache',context='SinglePoint',preliminary=preliminary,calls=[{},{}])
for name,value in [('nan',math.nan),('positive_inf',math.inf),('negative_inf',-math.inf)]:
 add('nonfinite_scope_probe','substance_'+name,policy='nonfinite_scope_probe',calls=[{'substance_f64_bits':bits(value)}])
 for label in ('floodedness','spread','preliminary'):
  changes={'endpoint':'fluid_scan','context':'SinglePoint','calls':[{},{}]}
  if label=='preliminary':changes['preliminary']={'default_f64_bits':bits(value),'overrides':[]}
  else:
   leaves=deepcopy(base['leaves']);leaves[label]=bits(value)
   if label=='spread':leaves['floodedness']=bits(0.5)
   changes['leaves']=leaves
  add('nonfinite_scope_probe',label+'_'+name,policy='nonfinite_scope_probe',**changes)
 leaves=deepcopy(base['leaves']);leaves['barrier']=bits(value)
 add('nonfinite_scope_probe','barrier_'+name,policy='nonfinite_scope_probe',endpoint='pressure',y=113,leaves=leaves,first={'height':116,'fluid':'water'},second={'height':100,'fluid':'water'},calls=[{'sentinel_f64_bits':bits(math.nan)},{'sentinel_f64_bits':bits(math.nan)}])
Path(__file__).with_name('cases.jsonl').write_text(''.join(json.dumps(row,separators=(',',':'))+'\n' for row in rows))
print('declared cases',len(rows))
