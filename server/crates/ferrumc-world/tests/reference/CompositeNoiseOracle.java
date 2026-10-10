// Calls the actual hash-pinned Minecraft classes; no noise/RNG algorithm copy.
package ferrumc.reference;

import java.io.PrintWriter;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.HexFormat;
import it.unimi.dsi.fastutil.doubles.DoubleArrayList;
import it.unimi.dsi.fastutil.doubles.DoubleList;

public final class CompositeNoiseOracle {
    static final Class<?> RNG=cls("bck"), SIMPLEX=cls("ezr"), NORMAL=cls("ezo"), PARAMS=cls("ezo$a"), BLENDED=cls("ezl"), PERLIN=cls("ezp"), IMPROVED=cls("ezm");
    static PrintWriter out;
    static long[] SEEDS={0,1,-1,42,Long.MIN_VALUE,Long.MAX_VALUE,1234567890123456789L,-9876543210L};
    static double[][] COORDS={
        {0,0,0},{-0.0,-0.0,-0.0},{Double.MIN_VALUE,-Double.MIN_VALUE,0},
        {.125,.25,.375},{.125,.375,.25},{.25,.125,.375},{.25,.375,.125},{.375,.125,.25},{.375,.25,.125},
        {.25,.25,.25},{.25,.25,.125},{.25,.125,.25},{.125,.25,.25},
        {-1,-1,-1},{-.125,-.25,-.375},{-1.0000000000000002,.9999999999999999,255.99999999999997},
        {256,319,-256},{29999999,-63.875,-29999999},{16777216,0,-16777216},
        {33554432.125,255.5,-33554432.125}
    };
    static int[][] BLOCKS={
        {0,0,0},{1,1,1},{-1,-64,-16},{16,319,16},{256,-1,-256},
        {30000000,0,-30000000},{-30000000,319,30000000},
        {Integer.MIN_VALUE,Integer.MAX_VALUE,-1},{Integer.MAX_VALUE,Integer.MIN_VALUE,1},
        {Integer.MIN_VALUE,Integer.MIN_VALUE,Integer.MIN_VALUE},{Integer.MAX_VALUE,Integer.MAX_VALUE,Integer.MAX_VALUE},
        {35,19,-71},{-17,80,33},{59,-17,-44},{1000,64,-1000},{137,90,251}
    };
    static double[][] CONFIGS={
        {.25,.125,80,160,8},{.25,.375,80,60,8},{.25,.25,80,160,4},
        {.001,.001,.001,.001,1},{1000,1000,1000,1000,8},
        {1000,.001,.001,1000,1},{.001,1000,1000,.001,8}
    };
    static Class<?> cls(String name) {try{return Class.forName(name,false,CompositeNoiseOracle.class.getClassLoader());}catch(Exception e){throw new RuntimeException(e);}}
    static Object call(Class<?> owner,Object receiver,String name,Class<?>[] types,Object...args)throws Exception{
        return owner.getMethod(name,types).invoke(receiver,args);
    }
    static Object field(Class<?> owner,Object receiver,String name)throws Exception{
        Field f=owner.getDeclaredField(name);f.setAccessible(true);return f.get(receiver);
    }
    static Object rng(String kind,long seed)throws Exception{return cls(kind.equals("L")?"ekb":"elb").getConstructor(long.class).newInstance(seed);}
    static long nextLong(Object random)throws Exception{return (Long)call(RNG,random,"g",new Class<?>[0]);}
    static String bits(double value){return String.format("%016x",Double.doubleToRawLongBits(value));}
    static String encoded(double[] values){StringBuilder result=new StringBuilder();for(double v:values){if(!result.isEmpty())result.append(',');result.append(bits(v));}return result.toString();}
    static String coords(double[] values){return bits(values[0])+"|"+bits(values[1])+"|"+bits(values[2]);}
    static String offsets(Class<?> owner,Object noise,String[] names)throws Exception{
        return bits((Double)field(owner,noise,names[0]))+","+bits((Double)field(owner,noise,names[1]))+","+bits((Double)field(owner,noise,names[2]));
    }
    static String permutation(Object noise)throws Exception{return HexFormat.of().formatHex((byte[])field(IMPROVED,noise,"e"));}
    static void stack(String row,String id,String label,Object stack,int count)throws Exception{
        for(int index=0;index<count;index++){
            Object noise=call(PERLIN,stack,"a",new Class<?>[]{int.class},index);
            out.println(row+"|"+id+"|"+label+"|"+index+"|"+(noise==null?"null":offsets(IMPROVED,noise,new String[]{"a","b","c"})+"|"+permutation(noise)));
        }
    }
    static Object blended(Object random,double[] config)throws Exception{
        return BLENDED.getConstructor(RNG,double.class,double.class,double.class,double.class,double.class)
            .newInstance(random,config[0],config[1],config[2],config[3],config[4]);
    }
    static double blendedValue(Object noise,int[] p)throws Exception{
        Object context=cls("ejt$e").getConstructor(int.class,int.class,int.class).newInstance(p[0],p[1],p[2]);
        return (Double)call(BLENDED,noise,"a",new Class<?>[]{cls("ejt$b")},context);
    }
    static void blendedRows(String id,String kind,long seed,double[] config,Object noise,Object random,
        String originKind,long originSeed,String originNext)throws Exception{
        out.println("B|"+id+"|"+kind+"|"+seed+"|"+encoded(config)+"|"+bits((Double)call(BLENDED,noise,"b",new Class<?>[0]))
            +"|"+bits((Double)call(BLENDED,noise,"a",new Class<?>[0]))+"|"+nextLong(random)+"|"+originKind+"|"+originSeed+"|"+originNext);
        stack("BO",id,"min",field(BLENDED,noise,"g"),16);
        stack("BO",id,"max",field(BLENDED,noise,"h"),16);
        stack("BO",id,"main",field(BLENDED,noise,"i"),8);
        for(int[] p:BLOCKS)out.println("BV|"+id+"|"+p[0]+"|"+p[1]+"|"+p[2]+"|"+bits(blendedValue(noise,p)));
    }
    public static void main(String[] args)throws Exception{
        call(cls("ac"),null,"a",new Class<?>[0]);
        call(cls("amg"),null,"a",new Class<?>[0]);
        out=new PrintWriter(Path.of(args[0]).toFile(),StandardCharsets.UTF_8);
        out.println("# Minecraft Java 1.21.8 actual original-JAR composite-noise reflection; exact raw f64 bits");
        int number=0;
        for(String kind:new String[]{"L","X"})for(long seed:SEEDS)for(int skip:new int[]{0,17292}){
            Object random=rng(kind,seed);call(RNG,random,"b",new Class<?>[]{int.class},skip);
            Object noise=SIMPLEX.getConstructor(RNG).newInstance(random);
            int[] original=(int[])field(SIMPLEX,noise,"h");byte[] used=new byte[256];
            for(int i=0;i<256;i++){used[i]=(byte)original[i];if(original[i+256]!=0)throw new IllegalStateException("unexpected unused permutation entry");}
            String id="s"+(number++);
            out.println("S|"+id+"|"+kind+"|"+seed+"|"+skip+"|"+offsets(SIMPLEX,noise,new String[]{"b","c","d"})+"|"+HexFormat.of().formatHex(used)+"|"+nextLong(random));
            for(double[] p:COORDS){
                out.println("S2|"+id+"|"+bits(p[0])+"|"+bits(p[1])+"|"+bits((Double)call(SIMPLEX,noise,"a",new Class<?>[]{double.class,double.class},p[0],p[1])));
                out.println("S3|"+id+"|"+coords(p)+"|"+bits((Double)call(SIMPLEX,noise,"a",new Class<?>[]{double.class,double.class,double.class},p[0],p[1],p[2])));
            }
            for(double[] p:new double[][]{{2147483136,2147483136,2147483136},{-2147483136,-2147483136,-2147483136},{2147483136,-2147483136,17.25}}){
                out.println("S2|"+id+"|"+bits(p[0])+"|"+bits(p[1])+"|"+bits((Double)call(SIMPLEX,noise,"a",new Class<?>[]{double.class,double.class},p[0],p[1])));
                out.println("S3|"+id+"|"+coords(p)+"|"+bits((Double)call(SIMPLEX,noise,"a",new Class<?>[]{double.class,double.class,double.class},p[0],p[1],p[2])));
            }
        }
        number=0;
        for(String kind:new String[]{"L","X"})for(long seed:SEEDS)for(boolean legacy:new boolean[]{false,true}){
            int[] firsts=legacy?new int[]{-3,-5,0,-64,-4,-4,-1}:new int[]{-3,0,-64,32,-63,-4,-1};
            for(int c=0;c<firsts.length;c++){
                double[] amps=c==0?new double[]{1,0,.5,1}:c==1?new double[]{0,-0.0,0}:c==4&&!legacy?new double[64]:c==5?new double[]{0,-0.0,0}:c==6?new double[]{-.25,1}:new double[]{1};
                if(c==4&&!legacy)for(int i=0;i<64;i++)amps[i]=i%3==0?0:i%3==1?1:-.25;
                Object random=rng(kind,seed);Object params=PARAMS.getConstructor(int.class,DoubleList.class).newInstance(firsts[c],new DoubleArrayList(amps));
                Object noise=call(NORMAL,null,legacy?"a":"b",new Class<?>[]{RNG,PARAMS},random,params);
                String id="n"+(number++);
                out.println("N|"+id+"|"+kind+"|"+seed+"|"+legacy+"|"+firsts[c]+"|"+encoded(amps)+"|"+bits((Double)call(NORMAL,noise,"a",new Class<?>[0]))+"|"+bits((Double)field(NORMAL,noise,"c"))+"|"+nextLong(random));
                stack("NO",id,"first",field(NORMAL,noise,"d"),amps.length);
                stack("NO",id,"second",field(NORMAL,noise,"e"),amps.length);
                for(double[] p:COORDS)out.println("NV|"+id+"|"+coords(p)+"|"+bits((Double)call(NORMAL,noise,"a",new Class<?>[]{double.class,double.class,double.class},p[0],p[1],p[2])));
                double cap=2147483136.0/1.0181268882175227;
                for(double[] p:new double[][]{{cap,cap,-cap},{Math.nextDown(cap),-Math.nextDown(cap),17.25}})
                    out.println("NV|"+id+"|"+coords(p)+"|"+bits((Double)call(NORMAL,noise,"a",new Class<?>[]{double.class,double.class,double.class},p[0],p[1],p[2])));
            }
        }
        number=0;
        for(String kind:new String[]{"L","X"})for(long seed:SEEDS)for(double[] config:CONFIGS){
            Object random=rng(kind,seed);Object noise=blended(random,config);String id="b"+(number++);
            blendedRows(id,kind,seed,config,noise,random,"-",0,"-");
        }
        for(String origin:new String[]{"L","X"})for(String kind:new String[]{"L","X"})for(long seed:new long[]{Long.MIN_VALUE,Long.MAX_VALUE})for(double[] config:new double[][]{CONFIGS[0],CONFIGS[2]}){
            Object source=rng(origin,-1);Object previous=blended(source,config);Object random=rng(kind,seed);
            Object noise=call(BLENDED,previous,"a",new Class<?>[]{RNG},random);String id="b"+(number++);
            blendedRows(id,kind,seed,config,noise,random,origin,-1,Long.toString(nextLong(source)));
            out.println("BA|"+id+"|"+bits(blendedValue(previous,BLOCKS[1])));
        }
        for(double[] endpoints:new double[][]{{-4.25,3.75},{-0.0,0.0},{1e16,Math.nextUp(1e16)}})
            for(double weight:new double[]{Math.nextDown(0.0),-0.0,0.0,Math.nextUp(0.0),.5,Math.nextDown(1.0),1.0,Math.nextUp(1.0),-1.0,2.0})
                out.println("CL|"+bits(endpoints[0])+"|"+bits(endpoints[1])+"|"+bits(weight)+"|"
                    +bits((Double)call(cls("bcb"),null,"b",new Class<?>[]{double.class,double.class,double.class},endpoints[0],endpoints[1],weight)));
        out.close();
        if(out.checkError())throw new IllegalStateException("fixture write failed");
    }
}
