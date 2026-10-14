import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.core.Direction;
import net.minecraft.world.phys.AABB;
import java.util.*;
import java.nio.file.*;
import java.io.*;
/** Exports immutable properties from the actual pinned Paper server, no server start. */
public final class ExportLight {
 public static void main(String[] args) throws Exception {
  SharedConstants.tryDetectVersion(); Bootstrap.bootStrap();
  var shapes=new ArrayList<byte[]>(); var ids=new HashMap<String,Integer>();
  var props=new ByteArrayOutputStream(); var output=new DataOutputStream(props);
  for(BlockState state:Block.BLOCK_STATE_REGISTRY){
   if(Block.getId(state)*13!=props.size())throw new IllegalStateException("state ordering");
   output.writeByte(state.getLightBlock() | state.getLightEmission()<<4);
   for(Direction direction:Direction.values()){
    byte[] bits=new byte[32];
    if(state.canOcclude()&&state.useShapeForLightOcclusion()){
     var boxes=state.getFaceOcclusionShape(direction).toAabbs();
     for(var box:boxes)for(double v:new double[]{box.minX,box.maxX,box.minY,box.maxY,box.minZ,box.maxZ})
      if(Math.abs(v*16-Math.rint(v*16))>1e-7)throw new IllegalStateException("off-grid shape "+state+" "+box);
     for(int a=0;a<16;a++)for(int b=0;b<16;b++){
      double u=(a+0.5)/16,v=(b+0.5)/16;
      double x=direction.getAxis()==Direction.Axis.X?0.5:u;
      double y=direction.getAxis()==Direction.Axis.Y?0.5:direction.getAxis()==Direction.Axis.X?u:v;
      double z=direction.getAxis()==Direction.Axis.Z?0.5:v;
      for(AABB box:boxes)if(x>box.minX&&x<box.maxX&&y>box.minY&&y<box.maxY&&z>box.minZ&&z<box.maxZ){int i=a*16+b;bits[i/8]|=(byte)(1<<(i%8));break;}
     }
    }
    String key=HexFormat.of().formatHex(bits); Integer id=ids.get(key);
    if(id==null){id=shapes.size(); ids.put(key,id);shapes.add(bits);}
    output.writeShort(id);
   }
  }
  Files.write(Path.of(args[0],"properties.bin"),props.toByteArray());
  var masks=new ByteArrayOutputStream();for(byte[] mask:shapes)masks.write(mask);
  Files.write(Path.of(args[0],"faces.bin"),masks.toByteArray());
  System.out.println("EXPORTED states="+props.size()/13+" face_masks="+shapes.size());
 }
}
