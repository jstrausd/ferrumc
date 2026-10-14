import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.levelgen.WorldgenRandom;
import net.minecraft.world.level.levelgen.XoroshiroRandomSource;
import java.nio.file.*;
public final class ExportOreRandom {
 public static void main(String[] args)throws Exception{
  SharedConstants.tryDetectVersion();Bootstrap.bootStrap();var text=new StringBuilder();
  for(long seed:new long[]{0,1,-1,Long.MIN_VALUE})for(int x:new int[]{0,-3})for(int z:new int[]{0,2})for(int index:new int[]{9,15,20,25,28}){
   var random=new WorldgenRandom(new XoroshiroRandomSource(0));long decoration=random.setDecorationSeed(seed,x*16,z*16);random.setFeatureSeed(decoration,index,6);
   text.append(seed+"\t"+x+"\t"+z+"\t"+index);
   for(int bound:new int[]{16,3,2,9,193,305,184,161,385})text.append("\t"+random.nextInt(bound));
   text.append("\t"+Integer.toUnsignedString(Float.floatToRawIntBits(random.nextFloat())));
   text.append("\t"+Long.toUnsignedString(Double.doubleToRawLongBits(random.nextDouble()))+"\n");
  }
  Files.writeString(Path.of(args[0]),text.toString());
 }
}
