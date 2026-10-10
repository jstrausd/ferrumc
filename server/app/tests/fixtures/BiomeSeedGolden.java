package fixture;
import java.lang.reflect.Method;
public class BiomeSeedGolden {
 public static void main(String[] args) throws Exception {
  Class<?> actualOriginalClass = Class.forName("dnz");
  Method originalMethod = actualOriginalClass.getDeclaredMethod("a", long.class);
  for (long seed : new long[]{0L, 1L, -1L, Long.MIN_VALUE, Long.MAX_VALUE, 0x0102030405060708L, 123456789L, -123456789L}) {
   System.out.println(seed + " " + originalMethod.invoke(null, seed));
  }
 }
}
