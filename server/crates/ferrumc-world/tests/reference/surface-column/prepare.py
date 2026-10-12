"""Declare complete GEN-05.10 inputs and freeze their whole bytes before original execution.

No numerical expectation is computed here. Groups, scan rules and selection predicates are
declared; every column input, write and outcome comes from actual original classes executed by
SurfaceColumnOracle.
"""
from pathlib import Path
import hashlib
import json
import re
import sys
import zipfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[3]
AREA = ROOT / '.codex-tmp/gen05_10'
HERE = AREA / 'oracle'
DESIGN = AREA / 'design'
BASE = ROOT / '.codex-tmp/gen05_10-root-base/archive'
BASE_FREEZE = ROOT / '.codex-tmp/gen05_10-root-base/base.freeze.json'
BASE_FREEZE_SHA = 'c6266dcb7e6be4608dde6f74beb7422079730fd9284117f5e1dc77d0c443321d'
BASE_FILES = 1386
ACCEPTED_08_PINS = ROOT / '.codex-tmp/gen05_8/oracle/input-pins.json'
ACCEPTED_08_PINS_SHA = 'bffbed18ac9f64025c5defcafb5df1b9ffec25191c2fb86c71ff319124fe50d8'
ACCEPTED_08_FREEZE = ROOT / '.codex-tmp/gen05_8/delivery/freeze-manifest.json'
ACCEPTED_08_FREEZE_SHA = 'd838de6c3af6d512d9b47741d8ed2be8b476337885bb0487d1a51435ea296cee'
ACCEPTED_09_FREEZE = ROOT / '.codex-tmp/gen05_9/round3/delivery/freeze-manifest.json'
ACCEPTED_09_FREEZE_SHA = '9f09cbd5bd191f3e173bf1843ab45fc5b5b9411de350fe45ff6a4429a8fa0e11'
ACCEPTED_09_HELPER = ROOT / '.codex-tmp/gen05_9/oracle/SurfaceRulesOracle.java'
MAPPINGS = ROOT / '.codex-tmp/gen05_2-root-reexport/server-mappings.txt'
ROSTER = ROOT / '.codex-tmp/gen05_7/census/metadata.json'


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
    path.parent.mkdir(parents=True, exist_ok=True)
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
    'HeightmapTypes': 'net.minecraft.world.level.levelgen.Heightmap$Types',
    'BlockState': 'net.minecraft.world.level.block.state.BlockState',
    'StateHolder': 'net.minecraft.world.level.block.state.StateHolder',
    'StateBase': 'net.minecraft.world.level.block.state.BlockBehaviour$BlockStateBase',
    'FluidState': 'net.minecraft.world.level.material.FluidState',
    'Block': 'net.minecraft.world.level.block.Block', 'Blocks': 'net.minecraft.world.level.block.Blocks',
    'Property': 'net.minecraft.world.level.block.state.properties.Property',
    'SurfaceRules': 'net.minecraft.world.level.levelgen.SurfaceRules',
    'RuleSource': 'net.minecraft.world.level.levelgen.SurfaceRules$RuleSource',
    'SurfaceRule': 'net.minecraft.world.level.levelgen.SurfaceRules$SurfaceRule',
    'Context': 'net.minecraft.world.level.levelgen.SurfaceRules$Context',
    'LazyCondition': 'net.minecraft.world.level.levelgen.SurfaceRules$LazyCondition',
    'ServerLevel': 'net.minecraft.server.level.ServerLevel', 'Level': 'net.minecraft.world.level.Level',
    'NormalNoise': 'net.minecraft.world.level.levelgen.synth.NormalNoise',
    'PositionalRandomFactory': 'net.minecraft.world.level.levelgen.PositionalRandomFactory',
    'XoroPositional': 'net.minecraft.world.level.levelgen.XoroshiroRandomSource$XoroshiroPositionalRandomFactory',
    'RandomSource': 'net.minecraft.util.RandomSource',
    'MultiNoise': 'net.minecraft.world.level.biome.MultiNoiseBiomeSource',
    'ParameterList': 'net.minecraft.world.level.biome.Climate$ParameterList',
    'RTree': 'net.minecraft.world.level.biome.Climate$RTree',
}
PAPER_ONLY = {
    'WorldConfiguration': 'io.papermc.paper.configuration.WorldConfiguration',
    'Environment': 'io.papermc.paper.configuration.WorldConfiguration$Environment',
}
# Overworld biomes the genuine MultiNoise overworld parameter list places at the surface. The three
# cave biomes (dripstone_caves, lush_caves, deep_dark) are depth-selected and are recorded only if a
# genuine column biome happens to be one of them.
SURFACE_BIOMES = sorted('''plains sunflower_plains snowy_plains ice_spikes desert swamp mangrove_swamp forest flower_forest
birch_forest dark_forest pale_garden old_growth_birch_forest old_growth_pine_taiga old_growth_spruce_taiga taiga snowy_taiga
savanna savanna_plateau windswept_hills windswept_gravelly_hills windswept_forest windswept_savanna jungle sparse_jungle
bamboo_jungle badlands eroded_badlands wooded_badlands meadow cherry_grove grove snowy_slopes frozen_peaks jagged_peaks
stony_peaks river frozen_river beach snowy_beach stony_shore warm_ocean lukewarm_ocean deep_lukewarm_ocean ocean
deep_ocean cold_ocean deep_cold_ocean frozen_ocean deep_frozen_ocean mushroom_fields'''.split())
CAVE_BIOMES = ['deep_dark', 'dripstone_caves', 'lush_caves']
SCAN = {'seed': 0, 'preset': 'normal', 'step_blocks': 256, 'radius_blocks': 49152,
        'block_y_levels': [64, 80, 100, 120, 150], 'order': 'chebyshev ring, then x, then z ascending',
        'hits_per_biome': 24, 'biome_candidates': 12, 'biome_accept_columns': 32}
# id, preset, seed, centre cx, cz. Chunk seams: the 2x2 quad around the origin and its +x/+z
# neighbours share four seams across the sign change of both axes.
FIXED_GROUPS = [
    ('origin/normal/0', 'normal', 0, 0, 0),
    ('seam/normal/0/+x', 'normal', 0, 1, 0),
    ('seam/normal/0/+z', 'normal', 0, 0, 1),
    ('negative/normal/0/-1,-1', 'normal', 0, -1, -1),
    ('negative/normal/0/-1,0', 'normal', 0, -1, 0),
    ('negative/normal/0/0,-1', 'normal', 0, 0, -1),
    ('negative/normal/0/-17,9', 'normal', 0, -17, 9),
    ('origin/normal/-1', 'normal', -1, 0, 0),
    ('origin/normal/i64min', 'normal', -9223372036854775808, 0, 0),
    ('origin/normal/i64max', 'normal', 9223372036854775807, 0, 0),
    ('origin/large_biomes/0', 'large_biomes', 0, 0, 0),
    ('origin/amplified/0', 'amplified', 0, 0, 0),
    ('far/normal/0/min', 'normal', 0, -1874998, -1874998),
    ('far/normal/0/max', 'normal', 0, 1874997, 1874997),
    # Original generation at a centre outside the FerrumC centre domain [-1874999, 1874998] (its +x
    # halo chunk 1875000 leaves the admitted chunk domain); the replay class must be a typed refusal.
    ('outside/normal/0/+x', 'normal', 0, 1874999, 0),
    # Genuine single-biome preset: overworld noise settings with a fixed plains biome source. FerrumC
    # policy P6 refuses a non-MultiNoise(minecraft:overworld) resolver; the class must be a typed refusal.
    ('policy/single_biome_surface/0', 'single_biome_surface', 0, 0, 0),
]
# Selection predicates evaluated on genuine Paper outputs of the instrumented SURFACE run (scan stage).
FEATURES = [
    # id, predicate, candidate biomes, max candidates, preset, candidate search
    ('eroded_pillar', 'at least one eroded-badlands write (air -> default block before the rule loop)', ['eroded_badlands'], 24, 'normal', 'hits'),
    ('iceberg_snow', 'at least one frozen-ocean SNOW_BLOCK write', ['frozen_ocean', 'deep_frozen_ocean'], 24, 'normal', 'hits'),
    ('iceberg_from_water', 'at least one frozen-ocean write onto a water block', ['frozen_ocean', 'deep_frozen_ocean'], 24, 'normal', 'hits'),
    ('iceberg_melt_true', 'a frozen-ocean column with writes whose genuine shouldMeltFrozenOceanIcebergSlightly is true', ['deep_frozen_ocean', 'frozen_ocean'], 24, 'normal', 'hits'),
    ('iceberg_melt_false', 'a frozen-ocean column with writes whose genuine shouldMeltFrozenOceanIcebergSlightly is false', ['frozen_ocean', 'deep_frozen_ocean'], 24, 'normal', 'hits'),
    ('frozen_hole', 'a frozen-ocean column with genuine surface depth <= 0 and a rule write of air, ice or water', ['frozen_ocean', 'deep_frozen_ocean'], 24, 'normal', 'hole'),
    ('rule_air_write', 'a rule write of air (frozen-ocean hole whose top stone has no water above); lowers WORLD_SURFACE_WG', ['frozen_ocean', 'deep_frozen_ocean'], 64, 'normal', 'hole'),
    ('swamp_water', 'a rule write of water (post-processing mark) in swamp or mangrove_swamp', ['swamp', 'mangrove_swamp'], 12, 'normal', 'hits'),
    ('lava_water_height', 'a rule call whose water height comes from a lava block', ['plains', 'forest', 'savanna', 'desert'], 12, 'normal', 'hits'),
    ('multi_run', 'a column with at least two separate stone runs', ['plains', 'windswept_hills'], 12, 'normal', 'hits'),
    ('top_320', 'a column whose genuine post-NOISE WORLD_SURFACE_WG height is 319', ['jagged_peaks', 'frozen_peaks', 'stony_peaks'], 24, 'amplified', 'hits'),
    ('steep_changed', 'a steep test reading a height that differs from the post-NOISE height', ['eroded_badlands', 'frozen_ocean', 'frozen_peaks'], 12, 'normal', 'hits'),
]
SEAM_PARTNERS = [('eroded_pillar', 1, 0), ('iceberg_snow', 0, 1)]
CASES = {
    'generation': 'Per group: genuine WorldPreset overworld generator, RandomState.create, nine ProtoChunks with level height '
                  '(-64, 384), cold RTree history, createBiomes row-major with a StructureManager over WorldOptions(seed, '
                  'generateStructures=false), persisted status BIOMES, genuine fillFromNoise on the centre (Blender.empty), '
                  'persisted status NOISE. Two independent generations per group (U and R) whose post-NOISE snapshots must be equal.',
    'u_run': 'Uninstrumented: genuine NoiseBasedChunkGenerator.buildSurface(ChunkAccess, WorldGenerationContext, RandomState, '
             'StructureManager, BiomeManager, Registry, Blender.empty) -- the callee of the WorldGenRegion overload.',
    'r_run': 'Recorded: genuine SurfaceSystem.buildSurface with the arguments NoiseBasedChunkGenerator passes and a '
             'transparent RuleSource proxy that applies the genuine settings.surfaceRule() and records every tryApply. '
             'U and R final chunks must be equal.',
    'biome_manager': 'Genuine BiomeManager(obfuscated seed) over a LevelReader proxy that serves exactly the nine BIOMES '
                     'chunks through LevelReader.getNoiseBiome (BIOMES,false), as WorldGenRegion does in the SURFACE step.',
    'world_generation_context': 'Paper: WorldGenerationContext(generator, LevelHeightAccessor(-64,384), Level carrier '
                                'with generateFlatBedrock=false); Vanilla: WorldGenerationContext(generator, accessor).',
    'diagnostics': 'After both runs, side-effect free genuine reads per column: the six SurfaceSystem extension noises at the '
                   'exact call arguments, a fresh noiseRandom.at(x,0,z) with nextInt(4), nextInt(10), four nextDouble, '
                   'Biome.shouldMeltFrozenOceanIcebergSlightly((x,seaLevel,z), seaLevel) on the column biome, and the '
                   'direct BiomeManager.getBiome((x, height+1, z)).',
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


def main():
    require(digest(BASE_FREEZE) == BASE_FREEZE_SHA, 'Root base freeze pin')
    baseline = json.loads(BASE_FREEZE.read_text())
    actual = {str(p.relative_to(BASE)): digest(p) for p in BASE.rglob('*') if p.is_file()}
    require(actual == baseline and len(actual) == BASE_FILES, 'Root whole archive membership/bytes')
    require(digest(ACCEPTED_08_PINS) == ACCEPTED_08_PINS_SHA, 'Accepted GEN-05.8 oracle pins')
    require(digest(ACCEPTED_08_FREEZE) == ACCEPTED_08_FREEZE_SHA, 'Accepted GEN-05.8 oracle freeze')
    require(digest(ACCEPTED_09_FREEZE) == ACCEPTED_09_FREEZE_SHA, 'Accepted GEN-05.9 contract freeze')
    frozen09 = json.loads(ACCEPTED_09_FREEZE.read_text())
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
    for p in (MAPPINGS, ROSTER, ACCEPTED_08_PINS, ACCEPTED_08_FREEZE, ACCEPTED_09_FREEZE, ACCEPTED_09_HELPER, BASE_FREEZE,
              ROOT / '.codex-tmp/gen05_10-root-base/baseline.json'):
        inventory[str(p)] = digest(p)
    symbols = mapping_symbols()
    write(HERE / 'semantic-mappings.properties', ''.join(k + '=' + v + '\n' for k, v in sorted(symbols.items())))
    write(HERE / 'aliases.txt', '\n'.join(sorted(CLASSES)) + '\n')
    write(HERE / 'paper-aliases.txt', '\n'.join(sorted(PAPER_ONLY)) + '\n')
    roster = sorted(json.loads(ROSTER.read_text())['full_biome_roster'])
    require(len(roster) == 65, 'Complete checked biome roster')
    require(set('minecraft:' + b for b in SURFACE_BIOMES + CAVE_BIOMES) <= set(roster) and len(SURFACE_BIOMES) == 51,
            'Declared surface biome list inside the roster')
    write(DESIGN / 'roster.tsv', '\n'.join(roster) + '\n')
    write(DESIGN / 'targets.tsv', 'biome\n' + ''.join('minecraft:' + t + '\n' for t in SURFACE_BIOMES))
    write(DESIGN / 'fixed-groups.tsv', 'group\tpreset\tseed\tcx\tcz\n' + ''.join('\t'.join(map(str, g)) + '\n' for g in FIXED_GROUPS))
    write(DESIGN / 'features.tsv', 'feature\tbiomes\tmax_candidates\tpredicate\tpreset\tsearch\n' +
          ''.join(f'{f}\t{",".join("minecraft:" + b for b in bs)}\t{n}\t{d}\t{pr}\t{s}\n' for f, d, bs, n, pr, s in FEATURES))
    write(DESIGN / 'seam-partners.tsv', 'feature\tdx\tdz\n' + ''.join(f'{f}\t{dx}\t{dz}\n' for f, dx, dz in SEAM_PARTNERS))
    contract = {'scope': 'Declared original inputs, groups, scan and selection predicates; no numerical expectation formula',
                'scan': SCAN, 'surface_biomes': SURFACE_BIOMES, 'cave_biomes': CAVE_BIOMES,
                'fixed_groups': [list(g) for g in FIXED_GROUPS],
                'features': [{'id': f, 'predicate': d, 'biomes': bs, 'max_candidates': n, 'preset': pr, 'search': s} for f, d, bs, n, pr, s in FEATURES],
                'hole_search': 'From each of the first six lattice hits of a candidate biome: 4-block square spiral up to 1536 blocks; '
                               'a column qualifies when genuine SurfaceSystem.getSurfaceDepth(x,z) <= 0 and the genuine MultiNoise '
                               'biome at y 40 or 64 is the candidate biome; its chunk becomes a candidate (distinct chunks, in spiral order).',
                'seam_partners': [list(s) for s in SEAM_PARTNERS], 'cases': CASES,
                'caps': {'groups': 96},
                'selection': 'Scan centres are lattice hits of genuine MultiNoiseBiomeSource outputs; candidates are verified '
                             'by genuine Paper generation and instrumented SURFACE runs. Selection uses original outputs; every '
                             'expectation remains an original outcome of the vanilla/paper runs.'}
    write(DESIGN / 'cases.json', json.dumps(contract, sort_keys=True, indent=2) + '\n')
    for mode in ('vanilla', 'paper'):
        origins = {}
        names = ['com.google.common.base.Suppliers', 'com.google.gson.JsonParser', 'com.mojang.serialization.JsonOps',
                 'com.mojang.serialization.Codec', 'com.mojang.serialization.Lifecycle',
                 'it.unimi.dsi.fastutil.longs.Long2IntOpenHashMap', 'it.unimi.dsi.fastutil.shorts.ShortList']
        for p in old['runtime_inputs'][mode]:
            with zipfile.ZipFile(p) as jar:
                contents = set(jar.namelist())
                for name in names:
                    if name.replace('.', '/') + '.class' in contents:
                        require(name not in origins, 'Duplicate library origin ' + name)
                        origins[name] = p
        require(set(origins) == set(names), 'Incomplete libraries ' + mode)
        write(HERE / (mode + '-library-origins.properties'), ''.join(k + '=' + v + '\n' for k, v in sorted(origins.items())))
    source_map = json.loads((AREA / 'sources/source-map.json').read_text())
    for rel, rec in source_map['files'].items():
        require(digest(AREA / 'sources' / rel) == rec['sha256'], 'Source copy drift ' + rel)
    for folder in (DESIGN, HERE, AREA / 'sources'):
        for p in folder.rglob('*'):
            if p.is_file() and p.name != 'input-pins.json' and '__pycache__' not in p.parts:
                inventory[str(p)] = digest(p)
    for p, h in inventory.items():
        require(digest(p) == h, 'Pre-execution input mismatch ' + p)
    pins = {'scope': 'GEN05.10 original preparation only', 'inventory': inventory, 'classes': CLASSES,
            'paper_only_classes': PAPER_ONLY, 'jdk': old['jdk'], 'jdk_inventory': current_jdk,
            'runtime_inputs': old['runtime_inputs'], 'root_archive_files': BASE_FILES, 'root_archive_manifest': baseline,
            'accepted_gen05_9_freeze_entries': len(frozen09.get('files', frozen09))}
    write(HERE / 'input-pins.json', json.dumps(pins, sort_keys=True, indent=2) + '\n')
    print(json.dumps({'inputs': len(inventory), 'symbols': len(symbols), 'pins_sha256': digest(HERE / 'input-pins.json')}, sort_keys=True))


if __name__ == '__main__':
    main()
