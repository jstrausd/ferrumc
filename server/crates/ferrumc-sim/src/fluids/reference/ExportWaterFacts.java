import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.core.Direction;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.EmptyBlockGetter;
import net.minecraft.world.level.block.*;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.material.*;
import net.minecraft.world.phys.AABB;
import ca.spottedleaf.moonrise.patches.collisions.block.CollisionBlockState;
import ca.spottedleaf.moonrise.patches.collisions.shape.CollisionVoxelShape;
import java.util.*;
import java.nio.file.*;
import java.io.*;
/** Actual pinned JVM facts. Dynamic/off-grid collisions are explicitly unadmitted. */
public final class ExportWaterFacts {
 public static void main(String[] args)throws Exception{
  SharedConstants.tryDetectVersion();Bootstrap.bootStrap();
  var any=FlowingFluid.class.getDeclaredMethod("canHoldAnyFluid",BlockState.class);any.setAccessible(true);
  var masks=new ArrayList<byte[]>();var ids=new HashMap<String,Integer>();
  var out=new ByteArrayOutputStream();var data=new DataOutputStream(out);int admitted=0;
  for(var state:Block.BLOCK_STATE_REGISTRY){
   var fluid=state.getFluidState();var block=state.getBlock();int flags=0;
   if(state.isSolid())flags|=2;if((boolean)any.invoke(null,state))flags|=4;
   int waterlog=-1;
   if(block instanceof LiquidBlockContainer container){
    flags|=8;
    if(container.canPlaceLiquid(null,EmptyBlockGetter.INSTANCE,BlockPos.ZERO,state,Fluids.WATER))flags|=16;
    if(container.canPlaceLiquid(null,EmptyBlockGetter.INSTANCE,BlockPos.ZERO,state,Fluids.FLOWING_WATER))flags|=32;
    if(block instanceof SimpleWaterloggedBlock && state.hasProperty(BlockStateProperties.WATERLOGGED))
     waterlog=Block.getId(state.setValue(BlockStateProperties.WATERLOGGED,true));
   }else flags|=16|32;
   if(fluid.getType()==Fluids.WATER||fluid.getType()==Fluids.FLOWING_WATER){
    flags|=64;if(fluid.isSource())flags|=128;if(fluid.getValue(FlowingFluid.FALLING))flags|=256;
   }else if(!fluid.isEmpty())flags|=512;
   boolean valid=!block.hasDynamicShape()||block==Blocks.WATER;
   if(((CollisionBlockState)state).moonrise$occludesFullBlock())flags|=1024;
   var shape=state.getCollisionShape(EmptyBlockGetter.INSTANCE,BlockPos.ZERO);
   var faces=new ArrayList<byte[]>();
   for(var direction:Direction.values()){
    var bits=new byte[32];var boxes=((CollisionVoxelShape)shape).moonrise$getFaceShapeClamped(direction).toAabbs();
    for(var box:boxes)for(double v:new double[]{box.minX,box.maxX,box.minY,box.maxY,box.minZ,box.maxZ})
     if(Math.abs(v*16-Math.rint(v*16))>1e-7)valid=false;
    for(int a=0;a<16;a++)for(int b=0;b<16;b++){
     double u=(a+.5)/16,v=(b+.5)/16;
     double x=direction.getAxis()==Direction.Axis.X?.5:u;
     double y=direction.getAxis()==Direction.Axis.Y?.5:direction.getAxis()==Direction.Axis.X?u:v;
     double z=direction.getAxis()==Direction.Axis.Z?.5:v;
     for(AABB box:boxes)if(x>box.minX&&x<box.maxX&&y>box.minY&&y<box.maxY&&z>box.minZ&&z<box.maxZ){int i=a*16+b;bits[i/8]|=(byte)(1<<(i%8));break;}
    }
    faces.add(bits);
   }
   if(valid){flags|=1;admitted++;}
   if(Block.getId(state)*20!=out.size())throw new IllegalStateException("state order");
   data.writeShort(flags);data.writeByte(fluid.getAmount());data.writeByte(0);data.writeInt(waterlog);
   for(var bits:faces){var key=HexFormat.of().formatHex(bits);var id=ids.get(key);if(id==null){id=masks.size();ids.put(key,id);masks.add(bits);}data.writeShort(id);}
  }
  Files.write(Path.of(args[0],"states.bin"),out.toByteArray());
  var faces=new ByteArrayOutputStream();for(var mask:masks)faces.write(mask);Files.write(Path.of(args[0],"faces.bin"),faces.toByteArray());
  System.out.println("states="+out.size()/20+" admitted="+admitted+" masks="+masks.size());
 }
}
