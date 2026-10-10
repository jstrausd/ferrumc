"""Declared boundary inputs only; expected values come exclusively from the JAR."""
from copy import deepcopy
import json
import math
from pathlib import Path
import struct


def b64(value):
    return struct.pack('>d', value).hex()


def b32(value):
    return struct.pack('>f', value).hex()


def f32(value):
    return struct.unpack('>f', struct.pack('>f', value))[0]


def neighbors64(value):
    return [('below', math.nextafter(value, -math.inf)), ('equal', value),
            ('above', math.nextafter(value, math.inf))]


def neighbors32(value):
    value = f32(value)
    bits = struct.unpack('>I', struct.pack('>f', value))[0]
    step = 1 if value >= 0 else -1
    return [('f32_below', struct.unpack('>f', struct.pack('>I', bits-step))[0]),
            ('f32_equal', value),
            ('f32_above', struct.unpack('>f', struct.pack('>I', bits+step))[0])]


BASE = {
    'x': 1, 'y': 25, 'z': 2, 'context': 'SinglePoint',
    'leaves': {key: b64(value) for key, value in {
        'barrier': 0.5, 'floodedness': 1.0, 'spread': 0.0, 'lava': 0.0,
        'erosion': 0.0, 'depth': 0.0, 'toggle': 0.8, 'ridged': -1.0, 'gap': 0.0,
    }.items()},
}
ROWS = []


def add(kind, name, *, leaves=None, floats=None, **values):
    inputs = deepcopy(BASE)
    inputs.update(values)
    if leaves:
        inputs['leaves'].update({key: b64(value) for key, value in leaves.items()})
    if floats is not None:
        inputs['float_bits'] = [b32(value) for value in floats]
    ROWS.append({'kind': kind, 'case': name, 'inputs': inputs})


water = {'height': 100, 'fluid': 'water'}
high_water = {'height': 116, 'fluid': 'water'}
lava = {'height': 116, 'fluid': 'lava'}
default_fluid = {'height': 63, 'fluid': 'water'}
sentinels = [('nan', '7ff8000000000000'), ('prefilled', b64(0.25))]

# The four pressure shapes at the nearest integral Y on both sides of the
# internal +/-2 guard. No synthetic pressure formula is evaluated here.
for label, y in [('upper_inside_positive2', 113), ('upper_outside_positive2',112),
                 ('lower_inside_positive2',102), ('lower_outside_positive2',103),
                 ('upper_inside_negative2',120), ('upper_outside_negative2',121),
                 ('lower_inside_negative2',77), ('lower_outside_negative2',76),
                 ('midpoint_below',107), ('midpoint_above',108),
                 ('upper_shape_split',115), ('upper_shape_after_split',116),
                 ('lower_shape_split',96), ('lower_shape_after_split',97)]:
    for sentinel_label, sentinel in sentinels:
        for calls in (1,2):
            add('pressure', f'{label}_{sentinel_label}_calls{calls}', y=y, context='NoiseChunk',
                first=high_water, second=water, sentinel_f64_bits=sentinel, calls=calls)
for name, first, second, y in [
    ('water_lava',water,lava,99), ('lava_water',lava,water,99),
    ('equal_water_heights',water,water,99),
    ('equal_height_water_lava',water,{'height':100,'fluid':'lava'},99),
    ('first_height_tie_air',water,lava,100), ('both_height_ties_air',water,water,100),
    ('second_height_tie_air',high_water,water,100),
]:
    add('pressure',name,y=y,context='NoiseChunk',first=first,second=second,
        sentinel_f64_bits='7ff8000000000000',calls=2)
for context in ('NoiseChunk','SinglePoint'):
    add('pressure',f'context_{context}',y=113,context=context,first=high_water,second=water,
        sentinel_f64_bits='7ff8000000000000',calls=2)
for barrier in (-0.0,0.0,-0.5,math.nextafter(0.5,math.inf)):
    add('pressure','barrier_'+b64(barrier),y=113,context='NoiseChunk',first=high_water,second=water,
        sentinel_f64_bits='7ff8000000000000',calls=2,leaves={'barrier':barrier})

# Deep-dark's constants are F32 values widened to F64. Include F64 neighbors,
# adjacent F32 values and the decimal F64 literals as separate input families.
erosions = neighbors64(f32(-0.225)) + neighbors32(-0.225) + [('decimal',-0.225)]
depths = neighbors64(f32(0.9)) + neighbors32(0.9) + [('decimal',0.9)]
for e_label, erosion in erosions:
    for d_label, depth in depths:
        add('surface_level',f'deepdark_e_{e_label}_d_{d_label}',
            fluid=default_fluid,max_surface=64,fluid_present=False,
            leaves={'erosion':erosion,'depth':depth,'floodedness':1.0})
for max_surface in (0,17,64,2_147_483_647,2_147_483_646,-2_147_483_648):
    for present in (False,True):
        for flooded in (-1.0,-0.8,-0.3,0.0,0.4,0.8,1.0):
            add('surface_level',f'surface_{max_surface}_present{int(present)}_flood_{b64(flooded)}',
                y=17,fluid=default_fluid,max_surface=max_surface,fluid_present=present,
                leaves={'floodedness':flooded,'spread':0.3})
for threshold in (-1.0,-0.8,-0.3,0.4,0.8,1.0):
    for label, flooded in neighbors64(threshold):
        add('surface_level',f'flood_threshold_{b64(threshold)}_{label}',
            fluid=default_fluid,max_surface=17,fluid_present=True,
            leaves={'floodedness':flooded,'spread':-0.3})

for label, spread in [('negative_zero',-0.0),('positive_zero',0.0),
                      *[(f'neg3_{key}',val) for key,val in neighbors64(-0.3)],
                      *[(f'pos3_{key}',val) for key,val in neighbors64(0.3)]]:
    for x,y,z in [(-17,-41,-17),(-16,-40,-16),(-1,-1,-1),(0,0,0),(15,39,15),(16,40,16),
                  (-30_000_000,-64,29_999_999),(29_999_999,319,-30_000_000)]:
        add('random_surface',f'spread_{label}_{x}_{y}_{z}',x=x,y=y,z=z,
            max_surface=2_147_483_647,leaves={'spread':spread})
for max_surface in (-100,-1,0,20,21,2_147_483_647):
    add('random_surface',f'spread_surface_clamp_{max_surface}',x=-1,y=0,z=16,
        max_surface=max_surface,leaves={'spread':0.3})

for sign in (-1,1):
    for label, value in neighbors64(sign*0.3)+neighbors32(sign*0.3):
        for level in (-9,-10,-11,-32512,-32511):
            add('fluid_type',f'lava_{sign}_{label}_level{level}',x=65,y=-41,z=-65,
                fluid=default_fluid,surface_level=level,leaves={'lava':value})
for name in ('water','lava','air'):
    for level in (-10,-32512):
        add('fluid_type',f'original_fluid_{name}_level{level}',x=-65,y=40,z=65,
            fluid={'height':63,'fluid':name},surface_level=level,leaves={'lava':0.8})

def ore(name, *, floats=(0.0,0.0,0.0), leaves=None, **values):
    add('ore',name,floats=floats,leaves=leaves,**values)

for toggle in (-0.0,0.0,math.nextafter(0.0,-math.inf),math.nextafter(0.0,math.inf)):
    ore('toggle_sign_'+b64(toggle),leaves={'toggle':toggle})
for sign in (-1,1):
    y = -34 if sign < 0 else 25
    for label, value in neighbors64(f32(0.4))+neighbors32(0.4):
        ore(f'veininess_{sign}_{label}',y=y,leaves={'toggle':sign*value})
    for label, value in neighbors64(f32(0.6))+neighbors32(0.6):
        ore(f'max_richness_toggle_{sign}_{label}',y=y,leaves={'toggle':sign*value})
for label,value in neighbors32(0.7):
    ore('solidness_'+label,floats=(value,0.0,0.0))
for label,value in [('negative_zero',-0.0),('positive_zero',0.0),
                    ('negative_nextafter',math.nextafter(0.0,-math.inf)),
                    ('positive_nextafter',math.nextafter(0.0,math.inf))]:
    ore('ridged_'+label,leaves={'ridged':value})
for threshold,toggle in [(0.1,f32(0.4)),(0.3,f32(0.6))]:
    for label,value in neighbors32(threshold):
        ore(f'richness_rng_{b32(threshold)}_{label}',leaves={'toggle':toggle},floats=(0.0,value,0.0))
for label,value in neighbors64(f32(-0.3))+neighbors32(-0.3):
    ore('gap_'+label,leaves={'gap':value})
for label,value in neighbors32(0.02):
    ore('raw_block_'+label,floats=(0.0,0.0,value))
for sign, ys in [(1,(-1,0,1,19,20,21,29,30,31,49,50,51)),
                 (-1,(-61,-60,-59,-41,-40,-39,-29,-28,-27,-9,-8,-7))]:
    for y in ys:
        ore(f'y_edge_{sign}_{y}',y=y,leaves={'toggle':sign*0.8})
for context in ('NoiseChunk','SinglePoint'):
    for family,values in [
        ('null_first_float',{'floats':(0.9,0.0,0.0)}),
        ('null_ridged',{'leaves':{'ridged':0.0}}),
        ('filler_richness',{'floats':(0.0,0.9,0.0)}),
        ('filler_gap',{'leaves':{'gap':-1.0}}),
        ('raw',{}),('ore',{'floats':(0.0,0.0,0.9)}),
    ]:
        ore(f'shortcircuit_{family}_{context}',context=context,**values)

# Actual original oreRandom streams additionally verify factory addressing and
# draw chronology without replacing nextFloat with declared values.
for seed in (0,1,-1,-9223372036854775808,9223372036854775807,81985529216486895,-81985529205931230):
    for x,y,z in ((1,25,2),(-17,-34,19),(29_999_999,25,-30_000_000)):
        add('ore',f'actual_seed_{seed}_{x}_{y}_{z}',x=x,y=y,z=z,actual_seed=str(seed),
            leaves={'toggle':-0.8 if y < 0 else 0.8})

for substance_label,substance in [('negative_zero',-0.0),('positive_zero',0.0),
                                ('positive_nextafter',math.nextafter(0.0,math.inf)),
                                ('negative_nextafter',math.nextafter(0.0,-math.inf))]:
    for name in ('water','lava','air'):
        for y in (62,63,64):
            add('disabled_substance',f'disabled_{substance_label}_{name}_y{y}',y=y,
                fluid={'height':63,'fluid':name},substance_f64_bits=b64(substance))

identities={(r['kind'],r['case']) for r in ROWS}
if len(identities)!=len(ROWS):
    raise ValueError('duplicate input identity')
if any(len(r['inputs'].get('float_bits',[]))>3 for r in ROWS):
    raise ValueError('declared float array exceeds cap')
Path(__file__).with_name('cases.jsonl').write_text(''.join(json.dumps(row,separators=(',',':'))+'\n' for row in ROWS))
print(json.dumps({'rows':len(ROWS),'groups':{kind:sum(r['kind']==kind for r in ROWS) for kind in sorted({r['kind'] for r in ROWS})}}))
