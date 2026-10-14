import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.server.level.*;
import net.minecraft.core.*;
import net.minecraft.world.flag.FeatureFlags;
import net.minecraft.world.level.*;
import net.minecraft.world.level.block.*;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.material.*;
import net.minecraft.world.level.chunk.*;
import java.util.*;
import java.nio.file.*;
import java.lang.reflect.*;
import sun.misc.Unsafe;
/** Invokes actual protected Paper fluid methods; no constructor or server starts. */
public final class ExportWaterRules {
 static Unsafe unsafe;
 static final BlockPos ZERO=BlockPos.ZERO;
 static class FixtureLevel extends ServerLevel {
  Map<BlockPos,BlockState> cells;GameRules rules;FixtureCache cache;
  private FixtureLevel(){super(null,null,null,null,null,null,null,false,0,List.of(),false,null,null,null,null);}
  public BlockState getBlockState(BlockPos pos){return cells.getOrDefault(pos,pos.getY()<0?Blocks.STONE.defaultBlockState():Blocks.AIR.defaultBlockState());}
  public FluidState getFluidState(BlockPos pos){return getBlockState(pos).getFluidState();}
  public GameRules getGameRules(){return rules;}
  public boolean isOutsideBuildHeight(BlockPos pos){return pos.getY() < -64 || pos.getY() >=320;}
  public boolean isOutsideBuildHeight(int y){return y < -64 || y>=320;}
  public int getMinY(){return -64;}
  public int getHeight(){return 384;}
 }
 static class FixtureCache extends ServerChunkCache {
  FixtureLevel level;Map<Long,FixtureChunk> chunks;
  private FixtureCache(){super(null,null,null,null,null,null,0,0,false,null,null,null);}
  public LevelChunk getChunkAtIfLoadedImmediately(int x,int z){return chunks.get(ChunkPos.asLong(x,z));}
 }
 static class FixtureChunk extends LevelChunk {
  FixtureLevel level;
  private FixtureChunk(){super(null,new ChunkPos(0,0));}
  public BlockState getBlockState(BlockPos pos){return level.getBlockState(pos);}
  public FluidState getFluidState(BlockPos pos){return level.getFluidState(pos);}
 }
 static FixtureLevel level(boolean conversion)throws Exception{
  var level=(FixtureLevel)unsafe.allocateInstance(FixtureLevel.class);level.cells=new HashMap<>();level.rules=new GameRules(FeatureFlags.DEFAULT_FLAGS);level.rules.getRule(GameRules.RULE_WATER_SOURCE_CONVERSION).set(conversion,null);
  var cache=(FixtureCache)unsafe.allocateInstance(FixtureCache.class);cache.level=level;cache.chunks=new HashMap<>();level.cache=cache;
  for(int x=-1;x<=0;x++)for(int z=-1;z<=0;z++){var chunk=(FixtureChunk)unsafe.allocateInstance(FixtureChunk.class);chunk.level=level;cache.chunks.put(ChunkPos.asLong(x,z),chunk);}
  var field=ServerLevel.class.getDeclaredField("chunkSource");field.setAccessible(true);field.set(level,cache);return level;
 }
 static void set(FixtureLevel l,int x,int y,int z,BlockState s){l.cells.put(new BlockPos(x,y,z),s);}
 public static void main(String[] args)throws Exception{
  SharedConstants.tryDetectVersion();Bootstrap.bootStrap();var uf=Unsafe.class.getDeclaredField("theUnsafe");uf.setAccessible(true);unsafe=(Unsafe)uf.get(null);
  var fresh=FlowingFluid.class.getDeclaredMethod("getNewLiquid",ServerLevel.class,BlockPos.class,BlockState.class);fresh.setAccessible(true);
  var spread=FlowingFluid.class.getDeclaredMethod("getSpread",ServerLevel.class,BlockPos.class,BlockState.class);spread.setAccessible(true);
  var text=new StringBuilder();var water=(FlowingFluid)Fluids.WATER;
  for(int kind=0;kind<12;kind++)for(boolean conversion:new boolean[]{false,true})for(int center:new int[]{0,1,4,7,8,15}){
   var l=level(conversion);set(l,0,0,0,Blocks.WATER.defaultBlockState().setValue(LiquidBlock.LEVEL,center));
   switch(kind){
    case 1 -> {set(l,-1,0,0,Blocks.WATER.defaultBlockState());set(l,1,0,0,Blocks.WATER.defaultBlockState());}
    case 2 -> {set(l,-1,0,0,Blocks.WATER.defaultBlockState());set(l,1,0,0,Blocks.WATER.defaultBlockState());set(l,0,-1,0,Blocks.AIR.defaultBlockState());}
    case 3 -> set(l,0,1,0,Blocks.WATER.defaultBlockState());
    case 4 -> {set(l,0,0,-1,Blocks.WATER.defaultBlockState().setValue(LiquidBlock.LEVEL,3));set(l,0,0,1,Blocks.WATER.defaultBlockState().setValue(LiquidBlock.LEVEL,6));}
    case 5 -> set(l,1,-1,0,Blocks.AIR.defaultBlockState());
    case 6 -> set(l,3,-1,0,Blocks.AIR.defaultBlockState());
    case 7 -> {set(l,5,-1,0,Blocks.AIR.defaultBlockState());set(l,-3,-1,0,Blocks.AIR.defaultBlockState());}
    case 8 -> {set(l,1,0,0,Blocks.STONE.defaultBlockState());set(l,0,0,-1,Blocks.STONE_SLAB.defaultBlockState());set(l,0,0,1,Blocks.OAK_TRAPDOOR.defaultBlockState());}
    case 9 -> {set(l,1,0,0,Blocks.WATER.defaultBlockState().setValue(LiquidBlock.LEVEL,7));set(l,-1,-1,0,Blocks.AIR.defaultBlockState());}
    case 10 -> {set(l,0,0,-1,Blocks.SHORT_GRASS.defaultBlockState());set(l,0,0,1,Blocks.OAK_SIGN.defaultBlockState());set(l,1,0,0,Blocks.STONE_SLAB.defaultBlockState());set(l,-1,0,0,Blocks.STONE_SLAB.defaultBlockState());set(l,-2,0,0,Blocks.WATER.defaultBlockState());}
    case 11 -> l.cache.chunks.remove(ChunkPos.asLong(-1,0));
   }
   var state=l.getBlockState(ZERO);var newLiquid=(FluidState)fresh.invoke(water,l,ZERO,state);
   @SuppressWarnings("unchecked") var result=(Map<Direction,FluidState>)spread.invoke(water,l,ZERO,state);
   text.append(kind+"\t"+conversion+"\t"+center+"\t");
   l.cells.entrySet().stream().sorted(Comparator.<Map.Entry<BlockPos,BlockState>>comparingInt(e->e.getKey().getX()).thenComparingInt(e->e.getKey().getY()).thenComparingInt(e->e.getKey().getZ())).forEach(e->{var p=e.getKey();text.append(p.getX()+","+p.getY()+","+p.getZ()+","+Block.getId(e.getValue())+";");});
   text.append("\t"+Block.getId(newLiquid.createLegacyBlock())+"\t");
   for(var entry:result.entrySet())text.append(entry.getKey().ordinal()+","+Block.getId(entry.getValue().createLegacyBlock())+";");
   text.append("\n");
  }
  Files.writeString(Path.of(args[0]),text.toString());System.out.println("actual new-liquid/spread cases=144");
 }
}
