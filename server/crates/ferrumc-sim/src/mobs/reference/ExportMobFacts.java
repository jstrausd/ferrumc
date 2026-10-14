import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.syncher.*;
import net.minecraft.world.entity.*;
import net.minecraft.world.entity.animal.*;
import net.minecraft.world.entity.animal.sheep.Sheep;
import net.minecraft.world.entity.monster.Zombie;
import net.minecraft.world.entity.ai.attributes.*;
import net.minecraft.world.damagesource.*;
import net.minecraft.world.level.EmptyBlockGetter;
import net.minecraft.world.level.block.*;
import java.util.*;
import java.nio.file.*;
import java.io.*;
/** Static facts from the pinned executable JVM. Bootstrap only; no world or events. */
public final class ExportMobFacts {
 static EntityType<? extends LivingEntity>[] types;
 static String accessor(Class<?> cls,String name)throws Exception{
  var f=cls.getDeclaredField(name);f.setAccessible(true);var a=(EntityDataAccessor<?>)f.get(null);
  return cls.getSimpleName()+"."+name+"\t"+a.id()+"\t"+EntityDataSerializers.getSerializedId(a.serializer())+"\n";
 }
 public static void main(String[] args)throws Exception{
  SharedConstants.tryDetectVersion();Bootstrap.bootStrap();types=new EntityType[]{EntityType.SHEEP,EntityType.COW,EntityType.PIG,EntityType.CHICKEN,EntityType.ZOMBIE};var dir=Path.of(args[0]);
  StringBuilder attrs=new StringBuilder();
  for(var type:types){var d=type.getDimensions();var a=DefaultAttributes.getSupplier(type);
   attrs.append(BuiltInRegistries.ENTITY_TYPE.getKey(type)).append('\t').append(BuiltInRegistries.ENTITY_TYPE.getId(type));
   for(double v:new double[]{d.width(),d.height(),d.eyeHeight(),a.getBaseValue(Attributes.MAX_HEALTH),a.getBaseValue(Attributes.MOVEMENT_SPEED),a.getBaseValue(Attributes.FOLLOW_RANGE),a.hasAttribute(Attributes.ATTACK_DAMAGE)?a.getBaseValue(Attributes.ATTACK_DAMAGE):0,a.getBaseValue(Attributes.ARMOR),a.getBaseValue(Attributes.STEP_HEIGHT)})attrs.append('\t').append(Double.toString(v));
   attrs.append('\n');
  }
  Files.writeString(dir.resolve("attributes.tsv"),attrs.toString());
  String meta=accessor(LivingEntity.class,"DATA_HEALTH_ID")+accessor(Mob.class,"DATA_MOB_FLAGS_ID")+accessor(AgeableMob.class,"DATA_BABY_ID")+accessor(Sheep.class,"DATA_WOOL_ID")+accessor(Pig.class,"DATA_BOOST_TIME")+accessor(Zombie.class,"DATA_BABY_ID")+accessor(Zombie.class,"DATA_SPECIAL_TYPE_ID")+accessor(Zombie.class,"DATA_DROWNED_CONVERSION_ID");
  Files.writeString(dir.resolve("metadata.tsv"),meta);
  var reach=Mob.class.getDeclaredField("DEFAULT_ATTACK_REACH");reach.setAccessible(true);Files.writeString(dir.resolve("attack-reach.txt"),Double.toString(reach.getDouble(null))+"\n");
  var states=new ByteArrayOutputStream();var data=new DataOutputStream(states);var shapes=new ArrayList<byte[]>();var ids=new HashMap<String,Integer>();int admitted=0,maxBoxes=0;
  for(var state:Block.BLOCK_STATE_REGISTRY){int flags=0;var block=state.getBlock();var boxes=state.getCollisionShape(EmptyBlockGetter.INSTANCE,BlockPos.ZERO).toAabbs();boolean valid=!block.hasDynamicShape()&&boxes.size()<=64;
   var buffer=new ByteArrayOutputStream();var shape=new DataOutputStream(buffer);shape.writeShort(boxes.size());
   for(var b:boxes)for(double v:new double[]{b.minX,b.minY,b.minZ,b.maxX,b.maxY,b.maxZ}){if(!Double.isFinite(v)||v < -1||v > 2)valid=false;shape.writeDouble(v);}
   byte[] bytes=buffer.toByteArray();var key=HexFormat.of().formatHex(bytes);Integer id=ids.get(key);if(id==null){id=shapes.size();ids.put(key,id);shapes.add(bytes);}
   if(valid){flags|=1;admitted++;}if(state.getFluidState().isEmpty())flags|=2;if(block==Blocks.GRASS_BLOCK)flags|=4;
   // prevent_mob_spawning_inside -> rails, checked in pinned resources. Bootstrap has no bound tags.
   boolean rail=block instanceof BaseRailBlock;
   // NodeEvaluator uses the FIRE tag; resolve its two pinned members explicitly.
   boolean fire=block==Blocks.FIRE||block==Blocks.SOUL_FIRE;
   for(int i=0;i<types.length;i++){
    if(state.isValidSpawn(EmptyBlockGetter.INSTANCE,BlockPos.ZERO,types[i]))flags|=1<<(3+i);
    if(!state.isCollisionShapeFullBlock(EmptyBlockGetter.INSTANCE,BlockPos.ZERO)&&!state.isSignalSource()&&state.getFluidState().isEmpty()&&!rail&&!types[i].isBlockDangerous(state)&&!fire)flags|=1<<(8+i);
   }
   if(!types[4].isBlockDangerous(state)&&!fire)flags|=8192;
   if(Block.getId(state)*4!=states.size())throw new IllegalStateException("state order");data.writeShort(flags);data.writeShort(id);maxBoxes=Math.max(maxBoxes,boxes.size());
  }
  if(shapes.size()>4096)throw new IllegalStateException("shape limit");Files.write(dir.resolve("states.bin"),states.toByteArray());var out=new ByteArrayOutputStream();var output=new DataOutputStream(out);output.writeShort(shapes.size());for(byte[] shape:shapes)out.write(shape);Files.write(dir.resolve("shapes.bin"),out.toByteArray());
  StringBuilder armor=new StringBuilder();var source=new DamageSource(null);
  for(float damage:new float[]{0.1F,1F,2F,3F,4F,8F,20F,100F})armor.append(Float.floatToRawIntBits(damage)).append('\t').append(Float.floatToRawIntBits(CombatRules.getDamageAfterAbsorb(null,damage,source,2,0))).append('\n');Files.writeString(dir.resolve("armor.tsv"),armor.toString());
  System.out.println("states="+states.size()/4+" admitted="+admitted+" shapes="+shapes.size()+" maxboxes="+maxBoxes+"\n"+attrs+meta);
 }
}
