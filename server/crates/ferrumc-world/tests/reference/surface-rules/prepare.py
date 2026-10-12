"""Declare complete GEN-05.9 inputs and freeze their whole bytes before original execution.

No numerical expectation is computed here. Case patterns are declared; every
outcome comes from actual original classes executed by SurfaceRulesOracle.
"""
from pathlib import Path
import decimal
import hashlib
import json
import re
import sys
import zipfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[3]
AREA = ROOT / '.codex-tmp/gen05_9'
HERE = AREA / 'oracle'
DESIGN = AREA / 'design'
BASE = ROOT / '.codex-tmp/gen05_9-root-base/archive'
BASE_FREEZE = ROOT / '.codex-tmp/gen05_9-root-base/base.freeze.json'
BASE_FREEZE_SHA = 'c9a6fd4ab13de587499219dee93e0f8c48f5c26eada956121b7a6fb8ac314c0e'
ACCEPTED_08_PINS = ROOT / '.codex-tmp/gen05_8/oracle/input-pins.json'
ACCEPTED_08_PINS_SHA = 'bffbed18ac9f64025c5defcafb5df1b9ffec25191c2fb86c71ff319124fe50d8'
ACCEPTED_08_FREEZE = ROOT / '.codex-tmp/gen05_8/delivery/freeze-manifest.json'
ACCEPTED_08_FREEZE_SHA = 'd838de6c3af6d512d9b47741d8ed2be8b476337885bb0487d1a51435ea296cee'
MAPPINGS = ROOT / '.codex-tmp/gen05_2-root-reexport/server-mappings.txt'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda: f.read(1048576), b''):
            h.update(block)
    return h.hexdigest()


def write(path, text):
    with path.open('x', encoding='utf-8', newline='\n') as f:
        f.write(text)


CLASSES = {
    'Shared': 'net.minecraft.SharedConstants', 'Bootstrap': 'net.minecraft.server.Bootstrap',
    'Util': 'net.minecraft.Util', 'TracingExecutor': 'net.minecraft.TracingExecutor',
    'VanillaRegistries': 'net.minecraft.data.registries.VanillaRegistries',
    'Provider': 'net.minecraft.core.HolderLookup$Provider', 'Lookup': 'net.minecraft.core.HolderLookup',
    'Getter': 'net.minecraft.core.HolderGetter', 'Holder': 'net.minecraft.core.Holder',
    'Reference': 'net.minecraft.core.Holder$Reference', 'Registry': 'net.minecraft.core.Registry',
    'MappedRegistry': 'net.minecraft.core.MappedRegistry', 'RegistrationInfo': 'net.minecraft.core.RegistrationInfo',
    'ResourceKey': 'net.minecraft.resources.ResourceKey', 'ResourceLocation': 'net.minecraft.resources.ResourceLocation',
    'Registries': 'net.minecraft.core.registries.Registries', 'BuiltInRegistries': 'net.minecraft.core.registries.BuiltInRegistries',
    'Biome': 'net.minecraft.world.level.biome.Biome', 'BiomeSource': 'net.minecraft.world.level.biome.BiomeSource',
    'ClimateSampler': 'net.minecraft.world.level.biome.Climate$Sampler',
    'Manager': 'net.minecraft.world.level.biome.BiomeManager',
    'NoiseSource': 'net.minecraft.world.level.biome.BiomeManager$NoiseBiomeSource',
    'Chunk': 'net.minecraft.world.level.chunk.ChunkAccess', 'ProtoChunk': 'net.minecraft.world.level.chunk.ProtoChunk',
    'Upgrade': 'net.minecraft.world.level.chunk.UpgradeData', 'Status': 'net.minecraft.world.level.chunk.status.ChunkStatus',
    'ChunkPos': 'net.minecraft.world.level.ChunkPos', 'BlockPos': 'net.minecraft.core.BlockPos',
    'MutableBlockPos': 'net.minecraft.core.BlockPos$MutableBlockPos',
    'Height': 'net.minecraft.world.level.LevelHeightAccessor', 'Reader': 'net.minecraft.world.level.LevelReader',
    'Accessor': 'net.minecraft.world.level.LevelAccessor',
    'BlendingData': 'net.minecraft.world.level.levelgen.blending.BlendingData',
    'Preset': 'net.minecraft.world.level.levelgen.presets.WorldPreset',
    'Dimensions': 'net.minecraft.world.level.levelgen.WorldDimensions',
    'Stem': 'net.minecraft.world.level.dimension.LevelStem',
    'ChunkGenerator': 'net.minecraft.world.level.chunk.ChunkGenerator',
    'Generator': 'net.minecraft.world.level.levelgen.NoiseBasedChunkGenerator',
    'Settings': 'net.minecraft.world.level.levelgen.NoiseGeneratorSettings',
    'RandomState': 'net.minecraft.world.level.levelgen.RandomState',
    'Blender': 'net.minecraft.world.level.levelgen.blending.Blender',
    'StructureManager': 'net.minecraft.world.level.StructureManager',
    'StructureCheck': 'net.minecraft.world.level.levelgen.structure.StructureCheck',
    'WorldOptions': 'net.minecraft.world.level.levelgen.WorldOptions',
    'SurfaceSystem': 'net.minecraft.world.level.levelgen.SurfaceSystem',
    'WGContext': 'net.minecraft.world.level.levelgen.WorldGenerationContext',
    'NoiseChunk': 'net.minecraft.world.level.levelgen.NoiseChunk',
    'Heightmap': 'net.minecraft.world.level.levelgen.Heightmap',
    'HeightmapTypes': 'net.minecraft.world.level.levelgen.Heightmap$Types',
    'BlockState': 'net.minecraft.world.level.block.state.BlockState',
    'StateHolder': 'net.minecraft.world.level.block.state.StateHolder',
    'StateBase': 'net.minecraft.world.level.block.state.BlockBehaviour$BlockStateBase',
    'Block': 'net.minecraft.world.level.block.Block', 'Blocks': 'net.minecraft.world.level.block.Blocks',
    'Property': 'net.minecraft.world.level.block.state.properties.Property',
    'DefaultedRegistry': 'net.minecraft.core.DefaultedRegistry',
    'CaveSurface': 'net.minecraft.world.level.levelgen.placement.CaveSurface',
    'SurfaceRules': 'net.minecraft.world.level.levelgen.SurfaceRules',
    'RuleSource': 'net.minecraft.world.level.levelgen.SurfaceRules$RuleSource',
    'ConditionSource': 'net.minecraft.world.level.levelgen.SurfaceRules$ConditionSource',
    'Condition': 'net.minecraft.world.level.levelgen.SurfaceRules$Condition',
    'SurfaceRule': 'net.minecraft.world.level.levelgen.SurfaceRules$SurfaceRule',
    'Context': 'net.minecraft.world.level.levelgen.SurfaceRules$Context',
    'LazyCondition': 'net.minecraft.world.level.levelgen.SurfaceRules$LazyCondition',
    'NotCondition': 'net.minecraft.world.level.levelgen.SurfaceRules$NotCondition',
    'TestRule': 'net.minecraft.world.level.levelgen.SurfaceRules$TestRule',
    'SequenceRule': 'net.minecraft.world.level.levelgen.SurfaceRules$SequenceRule',
    'StateRule': 'net.minecraft.world.level.levelgen.SurfaceRules$StateRule',
    'SequenceSrc': 'net.minecraft.world.level.levelgen.SurfaceRules$SequenceRuleSource',
    'TestSrc': 'net.minecraft.world.level.levelgen.SurfaceRules$TestRuleSource',
    'BlockSrc': 'net.minecraft.world.level.levelgen.SurfaceRules$BlockRuleSource',
    'BandlandsSrc': 'net.minecraft.world.level.levelgen.SurfaceRules$Bandlands',
    'BiomeCond': 'net.minecraft.world.level.levelgen.SurfaceRules$BiomeConditionSource',
    'NoiseCond': 'net.minecraft.world.level.levelgen.SurfaceRules$NoiseThresholdConditionSource',
    'GradientCond': 'net.minecraft.world.level.levelgen.SurfaceRules$VerticalGradientConditionSource',
    'YCond': 'net.minecraft.world.level.levelgen.SurfaceRules$YConditionSource',
    'WaterCond': 'net.minecraft.world.level.levelgen.SurfaceRules$WaterConditionSource',
    'NotCond': 'net.minecraft.world.level.levelgen.SurfaceRules$NotConditionSource',
    'StoneCond': 'net.minecraft.world.level.levelgen.SurfaceRules$StoneDepthCheck',
    'SteepCond': 'net.minecraft.world.level.levelgen.SurfaceRules$Steep',
    'HoleCond': 'net.minecraft.world.level.levelgen.SurfaceRules$Hole',
    'AboveCond': 'net.minecraft.world.level.levelgen.SurfaceRules$AbovePreliminarySurface',
    'TemperatureCond': 'net.minecraft.world.level.levelgen.SurfaceRules$Temperature',
    'Anchor': 'net.minecraft.world.level.levelgen.VerticalAnchor',
    'Absolute': 'net.minecraft.world.level.levelgen.VerticalAnchor$Absolute',
    'AboveBottom': 'net.minecraft.world.level.levelgen.VerticalAnchor$AboveBottom',
    'BelowTop': 'net.minecraft.world.level.levelgen.VerticalAnchor$BelowTop',
    'ServerLevel': 'net.minecraft.server.level.ServerLevel', 'Level': 'net.minecraft.world.level.Level',
    'NormalNoise': 'net.minecraft.world.level.levelgen.synth.NormalNoise',
    'PositionalRandomFactory': 'net.minecraft.world.level.levelgen.PositionalRandomFactory',
    'RandomSource': 'net.minecraft.util.RandomSource',
    'MultiNoise': 'net.minecraft.world.level.biome.MultiNoiseBiomeSource',
    'ParameterList': 'net.minecraft.world.level.biome.Climate$ParameterList',
    'RTree': 'net.minecraft.world.level.biome.Climate$RTree',
}
PAPER_ONLY = {
    'FlatBedrockCond': 'io.papermc.paper.world.worldgen.OptionallyFlatBedrockConditionSource',
    'WorldConfiguration': 'io.papermc.paper.configuration.WorldConfiguration',
    'Environment': 'io.papermc.paper.configuration.WorldConfiguration$Environment',
}
# Biomes named by the overworld surface rule plus temperature/default controls.
TARGETS = ['wooded_badlands', 'swamp', 'mangrove_swamp', 'badlands', 'eroded_badlands', 'frozen_ocean',
           'deep_frozen_ocean', 'frozen_peaks', 'snowy_slopes', 'jagged_peaks', 'grove', 'stony_peaks',
           'stony_shore', 'windswept_hills', 'warm_ocean', 'beach', 'snowy_beach', 'desert', 'dripstone_caves',
           'windswept_savanna', 'windswept_gravelly_hills', 'old_growth_pine_taiga', 'old_growth_spruce_taiga',
           'ice_spikes', 'mushroom_fields', 'lukewarm_ocean', 'deep_lukewarm_ocean', 'plains', 'snowy_taiga',
           'frozen_river', 'river', 'ocean', 'meadow', 'deep_dark', 'lush_caves']
SCAN = {'seed': 0, 'preset': 'normal', 'step_blocks': 256, 'radius_blocks': 49152,
        'block_y_levels': [64, 120, 176, 0, -40], 'order': 'chebyshev ring, then x, then z ascending'}
FIXED_GROUPS = [
    # id, preset, seed, center cx, cz, recording
    ('origin/normal/0', 'normal', 0, 0, 0, 'window'),
    ('origin/normal/-1', 'normal', -1, 0, 0, 'window'),
    ('origin/normal/i64min', 'normal', -9223372036854775808, 0, 0, 'window'),
    ('origin/normal/i64max', 'normal', 9223372036854775807, 0, 0, 'window'),
    ('negative/normal/0', 'normal', 0, -17, 9, 'window'),
    ('origin/large_biomes/0', 'large_biomes', 0, 0, 0, 'window'),
    ('origin/amplified/0', 'amplified', 0, 0, 0, 'window'),
]
SWEEP_FIRST_ONLY = ['warm_ocean', 'lukewarm_ocean', 'deep_lukewarm_ocean', 'desert', 'beach', 'snowy_beach', 'swamp',
                    'mangrove_swamp', 'mushroom_fields', 'ice_spikes', 'dripstone_caves']
SWEEP_TARGETS = ['wooded_badlands', 'badlands', 'eroded_badlands', 'frozen_peaks', 'snowy_slopes', 'grove', 'jagged_peaks',
                 'stony_peaks', 'stony_shore', 'windswept_savanna', 'windswept_gravelly_hills', 'windswept_hills',
                 'old_growth_pine_taiga', 'old_growth_spruce_taiga']
HOLE_TARGETS = ['frozen_ocean', 'deep_frozen_ocean', 'badlands', 'wooded_badlands', 'eroded_badlands']
WINDOW_TARGETS = ['wooded_badlands', 'eroded_badlands', 'frozen_ocean', 'frozen_peaks', 'desert', 'swamp',
                  'snowy_slopes', 'windswept_gravelly_hills']
CASES = {
    'g_window': 'G rows: every genuine buildSurface tryApply with y >= columnTop-24 (columnTop = WORLD_SURFACE_WG height + 1 at the first call)',
    'g_top': 'G rows for other groups: the first eight genuine tryApply calls of each column; every call enters the all-calls digest',
    'c_columns': [[0, 0], [15, 15], [0, 15], [15, 0], [7, 7], [3, 11], [11, 3], [5, 9], [9, 5], [1, 8], [8, 1], [13, 6]],
    'c_surface_offsets': list(range(-12, 13)),
    'c_surface_variants': [[1, 1, 'none'], [1, 1, 'y+1'], [1, 1, 'y'], [2, 5, 'none'], [4, 1, 'y-1'], [7, 2, 'y+7']],
    'c_gradient_y': list(range(-66, -55)) + list(range(-2, 11)),
    'c_anchor_y': [59, 60, 61, 62, 63, 64, 72, 73, 74, 75, 95, 96, 97, 98, 99, 254, 255, 256, 257],
    'c_anchor_variants': [[1, 1, 'none'], [1, 1, 'y+1'], [3, 2, 'none']],
    'c_end_y': [-2147483648, -65, -64, 319, 320, 2147483647],
    'c_stone_columns': [[0, 0], [7, 7], [15, 3]],
    'c_stone_depths': list(range(1, 46)),
    'c_water_columns': [[4, 4], [12, 12]],
    'c_water_offsets': list(range(-10, 11)),
    'c_water_stone': [1, 5],
    'steep_probe': 'one genuine ProtoChunk filled by original setBlockState with declared stone heights; '
                   'heights(x,z)=64+3*((x*7+z*3)%5)+((x+z)%2) for every local column',
    'paper_flat_bedrock': 'Paper JAR tree also evaluated under generateFlatBedrock=true for every C row with y <= 10',
    'cross_trees': 'Every C row also evaluated on separate contexts for the bootstrap and FerrumC-registry trees',
    'sweep': 'For groups of SWEEP_TARGETS/HOLE_TARGETS: every centre column, y in {top-1, top+2, 100, 110} kept only where the '
             'genuine BiomeManager biome equals the target, variants [[1,1,none],[1,2,y+1],[3,1,y+20]]; for y in {top-1, 100, 50} '
             '(50 only with these) also [[2,1,none],[4,2,none],[1,1,y+5],[1,2,y+20],[2,1,y+3],[1,1,y+20],[2,1,y+20],[2,2,none]]',
    'sweep_extra_hits': 2,
    'band_searches': [['wooded_badlands', 'minecraft:surface', 0.5454, 0.909, [100]],
                      ['stony_peaks', 'minecraft:calcite', -0.0125, 0.0125, [120, 64]],
                      ['grove', 'minecraft:powder_snow', 0.61, 1000000000.0, [120, 64]],
                      ['windswept_gravelly_hills', 'minecraft:surface', -1000000000.0, -0.13, [64, 100]],
                      ['wooded_badlands', 'minecraft:surface', 0.92, 1000000000.0, [100, 120]],
                      ['badlands', 'minecraft:surface', 0.92, 1000000000.0, [80, 100]]],
    'band_search': 'Around the first hit: x,z on a 4-block square spiral up to 1536 blocks; first column whose genuine '
                   'RandomState.getOrCreateNoise(noise).getValue(x,0,z) lies in [lo,hi] and whose genuine biome at one of '
                   'the listed y is the target; recorded and swept exactly like a hole group',
    'hole_search': 'Around the first hit: x,z on a 4-block square spiral up to 1536 blocks; first column whose genuine '
                   'SurfaceSystem.getSurfaceDepth(x,z) <= 0 and whose genuine biome at y 40 or 64 is the target',
    'hole_recording': 'G rows of the hole column only; C sweep plus every y in hole column top-12..top+12 with all eleven sweep variants',
}


def mapping_symbols():
    mapped, members, owner = {}, {}, None
    for line in MAPPINGS.read_text().splitlines():
        if line and not line[0].isspace() and not line.startswith('#'):
            owner, obfuscated = line[:-1].split(' -> ')
            mapped[owner] = obfuscated
            members[owner] = []
        elif line.startswith('    '):
            definition, obfuscated = line.strip().rsplit(' -> ', 1)
            definition = re.sub(r'^(?:\d+:\d+:)+', '', definition)
            members[owner].append((definition, obfuscated))
    symbols = {}
    for alias, canonical in CLASSES.items():
        symbols[f'vanilla.class.{alias}'] = mapped[canonical]
        symbols[f'paper.class.{alias}'] = canonical
        for signature, obfuscated in members[canonical]:
            if '(' in signature:
                signature = signature.split(' ', 1)[1]
                if signature.startswith('<') or '$' in signature.split('(')[0]:
                    continue
                symbols[f'vanilla.method.{alias}.{signature}'] = obfuscated
                symbols[f'paper.method.{alias}.{signature}'] = signature.split('(')[0]
            else:
                name = signature.rsplit(' ', 1)[-1]
                symbols[f'vanilla.field.{alias}.{name}'] = obfuscated
                symbols[f'paper.field.{alias}.{name}'] = name
    for alias, canonical in PAPER_ONLY.items():
        symbols[f'paper.class.{alias}'] = canonical
    return symbols


def exact_surface_rule(name):
    """Exact decimal tokens of the FerrumC registry's vendored export slice."""
    export = json.loads((BASE / 'docs/fixtures/minecraft-1.21.8-original/worldgen.json').read_text(),
                        parse_float=decimal.Decimal)
    entry = export['registries']['minecraft:worldgen/noise_settings']['entries']['minecraft:' + name]
    def encode(value):
        if isinstance(value, decimal.Decimal):
            return str(value)
        if isinstance(value, bool) or value is None:
            return json.dumps(value)
        if isinstance(value, (int, str)):
            return json.dumps(value)
        if isinstance(value, list):
            return '[' + ','.join(encode(v) for v in value) + ']'
        return '{' + ','.join(json.dumps(k) + ':' + encode(v) for k, v in value.items()) + '}'
    return encode(entry['surface_rule'])


def main():
    require(digest(BASE_FREEZE) == BASE_FREEZE_SHA, 'Root base freeze pin')
    baseline = json.loads(BASE_FREEZE.read_text())
    actual = {str(p.relative_to(BASE)): digest(p) for p in BASE.rglob('*') if p.is_file()}
    require(actual == baseline and len(actual) == 1292, 'Root whole archive membership/bytes')
    require(digest(ACCEPTED_08_PINS) == ACCEPTED_08_PINS_SHA, 'Accepted GEN-05.8 oracle pins')
    require(digest(ACCEPTED_08_FREEZE) == ACCEPTED_08_FREEZE_SHA, 'Accepted GEN-05.8 oracle freeze')
    old = json.loads(ACCEPTED_08_PINS.read_text())
    inventory = {str(BASE / name): value for name, value in actual.items()}
    for mode in ('vanilla', 'paper'):
        for name, value in old['runtime_inputs'][mode].items():
            require(digest(name) == value, 'Accepted original runtime artifact drift ' + name)
            inventory[name] = value
    current_jdk = {str(p): digest(p) for p in Path(old['jdk']).rglob('*') if p.is_file()}
    require(current_jdk == old['jdk_inventory'], 'Accepted complete JDK drift')
    inventory.update(current_jdk)
    require(digest(MAPPINGS) == old['inventory'][str(MAPPINGS)], 'Official mapping pin')
    inventory[str(MAPPINGS)] = digest(MAPPINGS)
    for p in (ACCEPTED_08_PINS, ACCEPTED_08_FREEZE, BASE_FREEZE, ROOT / '.codex-tmp/gen05_9-root-base/baseline.json'):
        inventory[str(p)] = digest(p)
    symbols = mapping_symbols()
    write(HERE / 'semantic-mappings.properties', ''.join(k + '=' + v + '\n' for k, v in sorted(symbols.items())))
    write(HERE / 'aliases.txt', '\n'.join(sorted(CLASSES)) + '\n')
    write(HERE / 'paper-aliases.txt', '\n'.join(sorted(PAPER_ONLY)) + '\n')
    for name in ('overworld', 'large_biomes', 'amplified'):
        write(DESIGN / ('registry-' + name + '-surface-rule.json'), exact_surface_rule(name) + '\n')
    write(DESIGN / 'targets.tsv', 'biome\n' + ''.join('minecraft:' + t + '\n' for t in TARGETS))
    write(DESIGN / 'fixed-groups.tsv', 'group\tpreset\tseed\tcx\tcz\trecording\n' +
          ''.join('\t'.join(map(str, g)) + '\n' for g in FIXED_GROUPS))
    roster = sorted(json.loads((ROOT / '.codex-tmp/gen05_7/census/metadata.json').read_text())['full_biome_roster'])
    require(len(roster) == 65, 'Complete checked biome roster')
    inventory[str(ROOT / '.codex-tmp/gen05_7/census/metadata.json')] = digest(ROOT / '.codex-tmp/gen05_7/census/metadata.json')
    write(DESIGN / 'roster.tsv', '\n'.join(roster) + '\n')
    contract = {'scope': 'Declared original inputs and case patterns; no numerical expectation formula',
                'scan': SCAN, 'targets': TARGETS, 'window_targets': WINDOW_TARGETS, 'sweep_targets': SWEEP_TARGETS,
                'sweep_first_only': SWEEP_FIRST_ONLY,
                'hole_targets': HOLE_TARGETS,
                'fixed_groups': [list(g) for g in FIXED_GROUPS], 'cases': CASES,
                'caps': {'groups': 96, 'rows_per_mode': 1000000},
                'selection': 'Scan centres are the first lattice hits of genuine MultiNoiseBiomeSource outputs; '
                             'group selection uses original outputs, every expectation remains an original outcome.'}
    write(DESIGN / 'cases.json', json.dumps(contract, sort_keys=True, indent=2) + '\n')
    for mode in ('vanilla', 'paper'):
        origins = {}
        names = ['com.google.common.base.Suppliers', 'com.google.gson.JsonParser', 'com.mojang.serialization.JsonOps',
                 'com.mojang.serialization.Codec', 'com.mojang.serialization.Lifecycle', 'it.unimi.dsi.fastutil.longs.Long2IntOpenHashMap']
        for p in old['runtime_inputs'][mode]:
            with zipfile.ZipFile(p) as jar:
                contents = set(jar.namelist())
                for name in names:
                    if name.replace('.', '/') + '.class' in contents:
                        require(name not in origins, 'Duplicate library origin ' + name)
                        origins[name] = p
        require(set(origins) == set(names), 'Incomplete libraries ' + mode)
        write(HERE / (mode + '-library-origins.properties'), ''.join(k + '=' + v + '\n' for k, v in sorted(origins.items())))
    for folder in (DESIGN, HERE, AREA / 'sources', AREA / 'bytecode'):
        for p in folder.rglob('*'):
            if p.is_file() and p.name != 'input-pins.json' and '__pycache__' not in p.parts:
                inventory[str(p)] = digest(p)
    source_map = json.loads((AREA / 'sources/source-map.json').read_text())
    for rel, rec in source_map['files'].items():
        require(digest(AREA / 'sources' / rel) == rec['sha256'], 'Source copy drift ' + rel)
    for p, h in inventory.items():
        require(digest(p) == h, 'Pre-execution input mismatch ' + p)
    pins = {'scope': 'GEN05.9 original preparation only', 'inventory': inventory, 'classes': CLASSES,
            'paper_only_classes': PAPER_ONLY, 'jdk': old['jdk'], 'jdk_inventory': current_jdk,
            'runtime_inputs': old['runtime_inputs'], 'root_archive_files': 1292, 'root_archive_manifest': baseline}
    write(HERE / 'input-pins.json', json.dumps(pins, sort_keys=True, indent=2) + '\n')
    print(json.dumps({'inputs': len(inventory), 'symbols': len(symbols), 'pins_sha256': digest(HERE / 'input-pins.json')}, sort_keys=True))


if __name__ == '__main__':
    main()
