"""Mechanical original predicates plus an explicit closed edge-stable terrain policy."""
from pathlib import Path
import struct,json
area=Path(__file__).resolve().parent.parent
resources=area.parent.parent.parent/'src/tree_resources'
STABLE=set('air cave_air void_air stone granite polished_granite diorite polished_diorite andesite polished_andesite bedrock deepslate calcite tuff dripstone_block water lava grass_block dirt coarse_dirt podzol rooted_dirt mycelium mud clay sand red_sand gravel sandstone red_sandstone snow snow_block powder_snow ice packed_ice blue_ice short_grass farmland coal_ore iron_ore copper_ore gold_ore redstone_ore lapis_ore diamond_ore emerald_ore deepslate_coal_ore deepslate_iron_ore deepslate_copper_ore deepslate_gold_ore deepslate_redstone_ore deepslate_lapis_ore deepslate_diamond_ore deepslate_emerald_ore raw_copper_block raw_iron_block terracotta'.split())
def main():
 rows=[v.split('\t') for v in (area/'original.tsv.facts').read_text().splitlines()[1:]];flags=[]
 for expected,r in enumerate(rows):
  if len(r)<14 or int(r[0])!=expected:raise ValueError('noncanonical state census')
  name=r[1].removeprefix('minecraft:');b=0
  for bit,col in [(0,2),(1,3),(2,4),(3,5),(4,6),(5,7),(6,8),(9,9),(10,12)]:
   if r[col]=='true':b|=1<<bit
   elif r[col]!='false':raise ValueError('nonboolean original predicate')
  if r[8]!=r[13]:raise ValueError('sapling species disagree')
  if name in ['grass_block','mycelium']:b|=1<<7
  if len(r)>14 and r[14]=='true':b|=1<<8
  if len(r)>15 and r[15]=='true':b|=1<<15
  if r[6]=='true':
   d=int(r[10]);b|=(d-1)<<11
   if [int(v) for v in r[11].split(',')]!=[expected+(new-d)*4 for new in range(1,8)]:raise ValueError('leaf variant layout drift')
  if name in STABLE or name.endswith('_terracotta') or r[5]=='true' or r[6]=='true':b|=1<<14
  flags.append(b)
 (resources/'state_facts.bin').write_bytes(struct.pack('<'+'H'*len(flags),*flags))
 (resources/'admitted_blocks.json').write_text(json.dumps(sorted('minecraft:'+n for n in STABLE),indent=2)+'\n')
 print(len(flags),'states;',sum(bool(v&(1<<14)) for v in flags),'admitted')
if __name__=='__main__':main()
