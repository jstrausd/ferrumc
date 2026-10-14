import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.block.Block;
import java.nio.file.*;
public final class ExportOreFacts {
 public static void main(String[] args)throws Exception{
  SharedConstants.tryDetectVersion();Bootstrap.bootStrap();
  byte[] values=new byte[Block.BLOCK_STATE_REGISTRY.size()];
  for(var state:Block.BLOCK_STATE_REGISTRY) values[Block.getId(state)]=(byte)(state.blocksMotion()?1:0);
  Files.write(Path.of(args[0]),values);
 }
}
