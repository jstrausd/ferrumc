// Standalone probe: reflection calls the hash-pinned ORIGINAL Minecraft JAR.
// No RNG or noise algorithm is implemented in this oracle.
package ferrumc.reference;

import java.io.PrintWriter;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.HexFormat;
import it.unimi.dsi.fastutil.doubles.DoubleArrayList;
import it.unimi.dsi.fastutil.doubles.DoubleList;

public final class RandomNoiseOracle {
    static final Class<?> RNG = cls("bck"), FACTORY = cls("ekn"), IMPROVED = cls("ezm"), PERLIN = cls("ezp");
    static PrintWriter out;
    static long[] SEEDS = {0, 1, -1, 42, Long.MIN_VALUE, Long.MAX_VALUE, 1234567890123456789L, -9876543210L};
    static double[][] COORDS = {{0,0,0}, {-0.0,-0.0,-0.0}, {0.125,-0.5,0.875}, {-1,-64,-16}, {-1.0000000000000002,0.9999999999999999,255.99999999999997}, {256,319,-256}, {29999999,-63.875,-29999999}, {16777216, 0, -16777216}, {33554432.125, 255.5,-33554432.125}, {2147483136, -2147483136, 17.25}};
    static double[][] VERTICAL = {{0,0}, {0.125,0.25}, {-0.25,-1}, {0.5,0}, {Double.MIN_VALUE,-1}};
    static Class<?> cls(String name) { try { return Class.forName(name); } catch (Exception e) { throw new RuntimeException(e); } }
    static Object call(Class<?> owner, Object receiver, String name, Class<?>[] types, Object... args) throws Exception { return owner.getMethod(name, types).invoke(receiver,args); }
    static Object rng(String kind,long seed) throws Exception { return cls(kind.equals("L")?"ekb":"elb").getConstructor(long.class).newInstance(seed); }
    static Object rc(Object random,String name) throws Exception { return call(RNG,random,name,new Class<?>[0]); }
    static String bits(double value) { return String.format("%016x",Double.doubleToRawLongBits(value)); }
    static String bits(float value) { return String.format("%08x",Float.floatToRawIntBits(value)); }
    static String coords(double[] v) { return bits(v[0])+"|"+bits(v[1])+"|"+bits(v[2]); }
    static Object improved(Object random) throws Exception { return IMPROVED.getConstructor(RNG).newInstance(random); }
    static String offsets(Object noise) throws Exception { return bits(IMPROVED.getField("a").getDouble(noise))+","+bits(IMPROVED.getField("b").getDouble(noise))+","+bits(IMPROVED.getField("c").getDouble(noise)); }
    static String permutation(Object noise) throws Exception { Field f=IMPROVED.getDeclaredField("e");f.setAccessible(true);return HexFormat.of().formatHex((byte[])f.get(noise)); }
    static void rrow(String kind,long seed,String op,Object arg,Object expected) { out.println("R|"+kind+"|"+seed+"|"+op+"|"+arg+"|"+expected); }
    public static void main(String[] args) throws Exception {
        out=new PrintWriter(Path.of(args[0]).toFile(),StandardCharsets.UTF_8);
        out.println("# Minecraft Java 1.21.8 original inner-JAR reflection oracle; f64 raw bits, no tolerances");
        for(long seed:SEEDS) {
            Object upgraded=call(cls("ekp"),null,"c",new Class<?>[]{long.class},seed);
            out.println("U|"+seed+"|"+call(cls("ekp$a"),upgraded,"b",new Class<?>[0])+"|"+call(cls("ekp$a"),upgraded,"c",new Class<?>[0]));
            for(String kind:new String[]{"L","X"}) {
                Object random=rng(kind,seed);
                for(int iteration=0;iteration<16;iteration++) {
                    rrow(kind,seed,"int",0,rc(random,"f"));rrow(kind,seed,"long",0,rc(random,"g"));
                    rrow(kind,seed,"bool",0,rc(random,"h"));rrow(kind,seed,"float",0,bits((Float)rc(random,"i")));rrow(kind,seed,"double",0,bits((Double)rc(random,"j")));
                    for(int bound:new int[]{1,2,3,7,255,256,257,1073741825,Integer.MAX_VALUE}) {
                        rrow(kind,seed,"bound",bound,call(RNG,random,"a",new Class<?>[]{int.class},bound));
                        rrow(kind,seed,"long",0,rc(random,"g"));
                    }
                }
                for(int count:new int[]{0,1,262,65536}) { call(RNG,random,"b",new Class<?>[]{int.class},count);rrow(kind,seed,"consume",count,rc(random,"g")); }
                Object fork=rc(random,"d");rrow(kind,seed,"fork",0,rc(fork,"g"));rrow(kind,seed,"long",0,rc(random,"g"));
                Object factory=rc(random,"e");Object origin=call(FACTORY,factory,"a",new Class<?>[]{int.class,int.class,int.class},0,0,0);
                rrow(kind,seed,"forkpos",0,rc(origin,"g"));rrow(kind,seed,"long",0,rc(random,"g"));
                call(RNG,random,"b",new Class<?>[]{long.class},seed);rrow(kind,seed,"reseed",seed,rc(random,"g"));
                for(int invalid:new int[]{0,-1,Integer.MIN_VALUE}) {
                    Object rejected=rng(kind,seed);
                    try { call(RNG,rejected,"a",new Class<?>[]{int.class},invalid);throw new IllegalStateException("invalid bound accepted"); }
                    catch(InvocationTargetException failure) {
                        if(!(failure.getCause() instanceof IllegalArgumentException))throw failure;
                        out.println("E|"+kind+"|"+seed+"|"+invalid+"|"+rc(rejected,"g"));
                    }
                }
                factory=rc(rng(kind,seed),"e");
                for(int[] p:new int[][]{{0,0,0},{-1,-64,-16},{16,319,16},{30000000,0,-30000000},{Integer.MIN_VALUE,Integer.MAX_VALUE,-1},{Integer.MAX_VALUE,Integer.MIN_VALUE,1}}) {
                    Object at=call(FACTORY,factory,"a",new Class<?>[]{int.class,int.class,int.class},p[0],p[1],p[2]);
                    out.println("P|"+kind+"|"+seed+"|at|"+p[0]+","+p[1]+","+p[2]+"|"+rc(at,"g")+"|"+bits((Double)rc(at,"j")));
                }
                for(String name:new String[]{"","octave_-64","octave_-1","octave_0","octave_32","minecraft:temperature","Grüße 💎\0","e\u0301","a".repeat(63),"a".repeat(64),"a".repeat(65)}) {
                    Object at=call(FACTORY,factory,"a",new Class<?>[]{String.class},name);
                    out.println("P|"+kind+"|"+seed+"|hash|"+HexFormat.of().formatHex(name.getBytes(StandardCharsets.UTF_8))+"|"+rc(at,"g")+"|"+bits((Double)rc(at,"j")));
                }
                for(long value:new long[]{0,-1,Long.MIN_VALUE,Long.MAX_VALUE}) {
                    Object at=call(FACTORY,factory,"a",new Class<?>[]{long.class},value);
                    out.println("P|"+kind+"|"+seed+"|seed|"+value+"|"+rc(at,"g")+"|"+bits((Double)rc(at,"j")));
                }
                random=rng(kind,seed);Object noise=improved(random);String id=kind+seed;
                out.println("I|"+id+"|"+kind+"|"+seed+"|"+offsets(noise)+"|"+permutation(noise)+"|"+rc(random,"g"));
                for(double[] p:COORDS) {
                    for(double[] y:VERTICAL) {
                        Object value=call(IMPROVED,noise,"a",new Class<?>[]{double.class,double.class,double.class,double.class,double.class},p[0],p[1],p[2],y[0],y[1]);
                        out.println("N|"+id+"|"+coords(p)+"|"+bits(y[0])+"|"+bits(y[1])+"|"+bits((Double)value));
                    }
                    double[] derivative={0,0,0};double value=(Double)call(IMPROVED,noise,"a",new Class<?>[]{double.class,double.class,double.class,double[].class},p[0],p[1],p[2],derivative);
                    out.println("D|"+id+"|"+coords(p)+"|"+bits(value)+"|"+bits(derivative[0])+"|"+bits(derivative[1])+"|"+bits(derivative[2]));
                }
            }
        }
        for(long[] halves:new long[][]{{0,0},{1,0},{0,1},{Long.MIN_VALUE,Long.MAX_VALUE}}) {
            Object random=cls("elb").getConstructor(long.class,long.class).newInstance(halves[0],halves[1]);
            for(int i=0;i<16;i++)out.println("Z|"+halves[0]+"|"+halves[1]+"|"+rc(random,"g"));
        }
        int idNumber=0;
        for(String kind:new String[]{"L","X"})for(long seed:new long[]{0,-1,Long.MIN_VALUE,Long.MAX_VALUE})for(boolean legacy:new boolean[]{false,true}) {
            int[] firsts=legacy?new int[]{-3,-5,0,-64}:new int[]{-3,0,-64,32,-63,-4};
            for(int c=0;c<firsts.length;c++) {
                double[] amps= c==0?new double[]{1,0,0.5,1}: c==1?(legacy?new double[]{1,0,0}:new double[]{0,-0.0,1}):c==4?new double[64]:c==5?new double[]{0,-0.0,0}:new double[]{c==2&&legacy?0:1};
                if(c==4)for(int i=0;i<64;i++)amps[i]=i%3==0?0:i%3==1?1:-0.25;
                Object random=rng(kind,seed);Object noise=call(PERLIN,null,legacy?"a":"b",new Class<?>[]{RNG,int.class,DoubleList.class},random,firsts[c],new DoubleArrayList(amps));
                Method max=PERLIN.getDeclaredMethod("a");max.setAccessible(true);
                StringBuilder encodedAmps=new StringBuilder();for(double a:amps){if(!encodedAmps.isEmpty())encodedAmps.append(',');encodedAmps.append(bits(a));}
                String id="p"+(idNumber++);out.println("C|"+id+"|"+kind+"|"+seed+"|"+legacy+"|"+firsts[c]+"|"+encodedAmps+"|"+bits((Double)max.invoke(noise))+"|"+rc(random,"g"));
                for(int index=0;index<amps.length;index++) {
                    Object octave=call(PERLIN,noise,"a",new Class<?>[]{int.class},index);
                    out.println("O|"+id+"|"+index+"|"+(octave==null?"null":offsets(octave)+"|"+permutation(octave)));
                }
                for(double multiplier:new double[]{0,-2,0.25,10})out.println("B|"+id+"|"+bits(multiplier)+"|"+bits((Double)call(PERLIN,noise,"a",new Class<?>[]{double.class},multiplier)));
                for(double[] p:COORDS)for(double[] y:VERTICAL)for(boolean fixed:new boolean[]{false,true}) {
                    double value=(Double)call(PERLIN,noise,"a",new Class<?>[]{double.class,double.class,double.class,double.class,double.class,boolean.class},p[0],p[1],p[2],y[0],y[1],fixed);
                    out.println("V|"+id+"|"+coords(p)+"|"+bits(y[0])+"|"+bits(y[1])+"|"+fixed+"|"+bits(value));
                }
            }
        }
        for(double x:new double[]{-0.0,0.0,-16777216.000000004,-16777216,-16777215.999999998,16777215.999999998,16777216,16777216.000000004,33554432.125,-33554432.125,2147483136,-2147483136}) {
            out.println("W|"+bits(x)+"|"+bits((Double)call(PERLIN,null,"b",new Class<?>[]{double.class},x)));
        }
        out.close();
    }
}
