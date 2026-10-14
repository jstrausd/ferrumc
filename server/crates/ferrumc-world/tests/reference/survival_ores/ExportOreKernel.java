import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.core.*;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.*;
import net.minecraft.world.level.*;
import net.minecraft.world.level.biome.*;
import net.minecraft.world.level.block.*;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.chunk.*;
import net.minecraft.world.level.levelgen.*;
import net.minecraft.world.level.levelgen.feature.*;
import net.minecraft.world.level.levelgen.feature.configurations.*;
import net.minecraft.world.level.levelgen.structure.templatesystem.BlockMatchTest;
import com.mojang.serialization.Lifecycle;
import java.util.*;
import java.lang.reflect.Proxy;
import java.nio.file.*;
/** Controlled solid/air boundary inputs to the actual complete Paper OreFeature. */
public final class ExportOreKernel {
 public static void main(String[] args)throws Exception{
  SharedConstants.tryDetectVersion();Bootstrap.bootStrap();
  var biomes=new MappedRegistry<Biome>(Registries.BIOME,Lifecycle.stable());
  var biome=new Biome.BiomeBuilder().temperature(.8f).downfall(.4f).mobSpawnSettings(MobSpawnSettings.EMPTY).generationSettings(BiomeGenerationSettings.EMPTY).specialEffects(new BiomeSpecialEffects.Builder().fogColor(0).waterColor(0).waterFogColor(0).skyColor(0).build()).build();
  Registry.register(biomes,Biomes.PLAINS,biome);biomes.freeze();
  var height=LevelHeightAccessor.create(-64,384);var text=new StringBuilder();
  for(long seed:new long[]{0,1,2,999})for(int size:new int[]{4,7,17,20})for(int kind:new int[]{0,1,2,3})for(float discard:new float[]{0,.5f,1}){
   BlockState substrate=kind==1?Blocks.DEEPSLATE.defaultBlockState():kind==3?Blocks.BEDROCK.defaultBlockState():Blocks.STONE.defaultBlockState();
   BlockState target=kind==1?Blocks.DEEPSLATE_DIAMOND_ORE.defaultBlockState():Blocks.DIAMOND_ORE.defaultBlockState();
   var chunks=new HashMap<Long,ProtoChunk>();
   java.util.function.BiFunction<Integer,Integer,ProtoChunk> get=(x,z)->chunks.computeIfAbsent(ChunkPos.asLong(x,z),key->{
    var chunk=new ProtoChunk(new ChunkPos(x,z),UpgradeData.EMPTY,height,biomes,null);
    for(var section:chunk.getSections())for(int lx=0;lx<16;lx++)for(int ly=0;ly<16;ly++)for(int lz=0;lz<16;lz++) section.setBlockState(lx,ly,lz,kind==2&&x*16+lx==0?Blocks.AIR.defaultBlockState():substrate,false);
    return chunk;
   });
   WorldGenLevel level=(WorldGenLevel)Proxy.newProxyInstance(ExportOreKernel.class.getClassLoader(),new Class<?>[]{WorldGenLevel.class},(proxy,method,a)->switch(method.getName()){
    case "getHeight" -> 320;
    case "getMinY" -> -64;
    case "getSectionsCount" -> 24;
    case "getSectionIndex" -> (((int)a[0])+64)>>4;
    case "isOutsideBuildHeight" -> ((int)a[0]) < -64 || ((int)a[0])>=320;
    case "ensureCanWrite" -> true;
    case "getChunk" -> get.apply((int)a[0],(int)a[1]);
    default -> throw new UnsupportedOperationException("unhandled "+method);
   });
   var random=new WorldgenRandom(new XoroshiroRandomSource(0));long decoration=random.setDecorationSeed(seed,0,0);random.setFeatureSeed(decoration,18,6);
   var config=new OreConfiguration(new BlockMatchTest(kind==1?Blocks.DEEPSLATE:Blocks.STONE),target,size,discard);
   boolean placed=Feature.ORE.place(new FeaturePlaceContext<>(Optional.empty(),level,null,random,new BlockPos(0,8,0),config));
   var positions=new ArrayList<BlockPos>();for(var chunk:chunks.values())for(int y=-64;y<320;y++)for(int x=0;x<16;x++)for(int z=0;z<16;z++){
    var p=new BlockPos(chunk.getPos().x*16+x,y,chunk.getPos().z*16+z);
    if(chunk.getBlockState(p)==target)positions.add(p);
   }
   positions.sort(Comparator.<BlockPos>comparingInt(p -> p.getX()).thenComparingInt(p -> p.getY()).thenComparingInt(p -> p.getZ()));
   text.append(seed+"\t"+size+"\t"+kind+"\t"+discard+"\t"+placed+"\t"+random.getCount()+"\t");
   for(var p:positions)text.append(p.getX()+","+p.getY()+","+p.getZ()+";");text.append("\n");
  }
  Files.writeString(Path.of(args[0]),text.toString());
 }
}
