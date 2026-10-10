package ferrumc.independent;

import com.google.gson.*;
import java.nio.file.*;
import java.io.*;
import java.util.*;
import static ferrumc.independent.NoiseChunkIndependentOracle.*;

/** Declared finite inputs only; actual original methods decide all branches. */
public final class FiniteNoiseChunkArrays {
    public static void main(String[] args) throws Exception {
        if(args.length!=6)throw new IllegalArgumentException("MODE MAPPINGS OUTPUT INNER CLASSES LIBRARY_ORIGINS required");
        paper=args[0].equals("paper");
        inner=Path.of(args[3]);
        mappings=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        origins=new JsonObject();
        for(var entry:mappings.entrySet())reverse.put(paper?entry.getKey():entry.getValue().getAsJsonObject().get("class").getAsString(),entry.getKey());
        requireOrigin(NoiseChunkIndependentOracle.class,Path.of(args[4]));
        var self=FiniteNoiseChunkArrays.class.getProtectionDomain().getCodeSource();
        if(self==null||!Files.isSameFile(Path.of(self.getLocation().toURI()),Path.of(args[4])))throw new IllegalStateException("wrong finite addendum CodeSource");
        origins.addProperty("ferrumc.independent.FiniteNoiseChunkArrays","$fresh-helper-classes");
        System.out.println("verified actual CodeSource finite addendum $fresh-helper-classes");
        JsonObject libraries=JsonParser.parseString(Files.readString(Path.of(args[5]))).getAsJsonObject();
        for(var entry:libraries.entrySet())requireOrigin(Class.forName(entry.getKey()),Path.of(entry.getValue().getAsString()));
        for(String cls:List.of("DensityFunction", "DensityFunction$FunctionContext", "DensityFunction$ContextProvider", "DensityFunction$SinglePointContext", "DensityFunctions", "DensityFunctions$Ap2", "DensityFunctions$TwoArgumentSimpleFunction", "DensityFunctions$MulOrAdd", "DensityFunctions$RangeChoice", "DensityFunctions$PureTransformer", "DensityFunctions$TransformerWithContext", "DensityFunctions$BlendDensity", "DensityFunctions$Mapped", "DensityFunctions$Clamp", "DensityFunctions$Constant", "Blender", "Mth"))requireOrigin(type(cls),inner);
        call(null,"SharedConstants","tryDetectVersion");
        call(null,"Bootstrap","bootStrap");
        try(PrintWriter stream=new PrintWriter(Files.newBufferedWriter(Path.of(args[2])))) {
            output=stream;
            finiteArrays();
            JsonObject row=row("origins","after-all-finite-array-calls");row.add("code_sources",origins);write(row);
        }
    }
    static void finiteArrays() throws Exception {
        double[] inputs={0,-0.0,-3,Math.nextDown(2.0),2,Math.nextUp(2.0),5,6.0};
        for(String operation:List.of("add","mul","min","max"))for(String failure:List.of("none","left","right")){
            JsonArray trace=new JsonArray();Probe left=new Probe("left",trace,null,-4,6,inputs),right=new Probe("right",trace,null,2,2,11);
            if(failure.equals("left"))left.failFill=true;if(failure.equals("right"))right.failAt=1;
            Object function=call(null,"DensityFunctions",operation,new String[]{"DensityFunction","DensityFunction"},left.function,right.function);
            observation("array",operation+"|"+failure,null,trace,()->{
                double[] a=new double[inputs.length];Arrays.fill(a,-99);
                try{fill(function,a,provider(trace));}finally{JsonObject r=row("array-prefix","actual");r.add("bits",doubles(a));trace.add(r);}return a;
            });
        }
        for(double zero:new double[]{0,-0.0})for(boolean fail:new boolean[]{false,true}){
            JsonArray trace=new JsonArray();Probe input=new Probe("input",trace,null,-5,5,inputs);input.failFill=fail;
            Object function=call(null,"DensityFunctions","mul",new String[]{"DensityFunction","DensityFunction"},constant(zero),input.function);
            observation("array","mulOrAdd-zero|"+bits(zero)+"|"+fail,null,trace,()->{double[] a=new double[inputs.length];try{fill(function,a,provider(trace));}finally{JsonObject r=row("array-prefix","actual");r.add("bits",doubles(a));trace.add(r);}return a;});
        }
        for(String failure:List.of("none","inside","outside","input")){
            JsonArray trace=new JsonArray();Probe input=new Probe("input",trace,null,-3,5,inputs),inside=new Probe("inside",trace,null,17,17,17),outside=new Probe("outside",trace,null,-19,-19,-19);
            if(failure.equals("input"))input.failFill=true;if(failure.equals("inside"))inside.failAt=0;if(failure.equals("outside"))outside.failAt=0;
            Object function=call(null,"DensityFunctions","rangeChoice",new String[]{"DensityFunction","double","double","DensityFunction","DensityFunction"},input.function,0.0,2.0,inside.function,outside.function);
            observation("array","rangeChoice|"+failure,null,trace,()->{double[] a=new double[inputs.length];try{fill(function,a,provider(trace));}finally{JsonObject r=row("array-prefix","actual");r.add("bits",doubles(a));trace.add(r);}return a;});
        }
        for(String operation:List.of("blendDensity","abs","clamp")){
            JsonArray trace=new JsonArray();Probe input=new Probe("input",trace,null,-5,5,inputs);
            Object function=operation.equals("blendDensity")?call(null,"DensityFunctions","blendDensity",new String[]{"DensityFunction"},input.function):
                operation.equals("abs")?call(input.function,"DensityFunction","abs"):call(input.function,"DensityFunction","clamp",new String[]{"double","double"},-1.0,1.0);
            observation("array",operation+"|context-order",null,trace,()->{double[] a=new double[inputs.length];fill(function,a,provider(trace));return a;});
        }
    }
}
