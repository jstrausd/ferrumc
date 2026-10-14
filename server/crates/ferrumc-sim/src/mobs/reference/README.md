# Pinned adult mob subset

Paper 1.21.8 build 60, applied checkout commit
`29c8822d90899c89d2689338e81a98f690bcba12`, inner executable jar SHA-256
`c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`.

`ExportMobFacts.java` bootstraps that genuine JVM and calls the original
DefaultAttributes suppliers, entity dimensions, metadata accessor objects,
block collision shapes/isValidSpawn predicates, dangerous-block predicates,
and CombatRules armor method. It exports all 27,946 state facts with 321 deduplicated
binary64 shape lists (largest 15 boxes); 27,767 states have admitted static
shapes. Dynamic shape states are closed. Original cache initialization rejects
nonempty static collision shapes that have coordinate offset functions.
No Java server, world events, full AI tick, navigation, or full-server
differential is represented by this instrumentation.

Bootstrap does not bind resource tags. Three exact pinned resource expansions
are applied explicitly: animals_spawnable_on contains grass_block;
prevent_mob_spawning_inside resolves rails to rail/powered_rail/detector_rail/
activator_rail; fire contains fire and soul_fire. All five admitted entity
forms are ordinary fire-vulnerable types. Other dangerous-block checks use the
original EntityType/NodeEvaluator methods.

## Integration contract

Only `mobs.rs` and `mobs/**` are supplied. The Sim owner adds its module declaration
and integration. No existing lib, shard, message or persistence file is modified.
Canonical EntityData/EntityStore remains the sole transform owner.

1. Obtain `MobRules::builtin()`. Use `attributes(kind).entity_type()` for genuine
   wire ids. Exposed metadata accessors carry original index + serializer id.
   Initial health is original max health; mob flags/wool/boost/special are zero;
   adult baby and conversion flags are false. Remaining initial entity metadata
   follows existing canonical protocol defaults. Current health goes in accessor9,
   serializer3. Supported sheep are white/unsheared; other forms are adult,
   unequipped/unsaddled/non-jockey default variants.
2. For ordinary dry Overworld spawning, supply current validated resident light
   (`MobSpawnLight`), nearest eligible spawn-affecting player distance squared,
   difficulty, and original-sized sky/dark RNG rolls through `MobSpawnContext`.
   `validate_spawn` checks >24 and <=128 blocks, original valid floor/spawn-cell
   facts, grass + raw>8 for passives, original default Overworld zombie block
   light0 + sky/dark rolls + non-Peaceful, headroom/body/support and resident cells.
   Owner separately checks world border, shared spawn exclusion24, biome/category
   choices, population caps, entity obstruction and admission/output capacity.
3. Create the canonical entity through its normal tick owner APIs, then
   `MobController::admit(id,kind,seed)`. Cap64 includes retained dead admissions.
   Removal uses `remove(id)` and returns its final health fact for owner retention.
   This controller is not a persistence reload codec; caller must not claim
   persistent AI/entities merely by storing health facts.
4. Call `step(now,&actors,&targets,read)` every owner tick; after the first tick,
   only exact successors are accepted. Snapshots use checked `MobActor::new` and
   `MobTarget::new`. Caller excludes creative/spectator/dead/disconnected and
   unsupported player poses. One call has <=64 actors/targets; duplicate/unknown
   preflight refusal leaves all brains unchanged. A successful step publishes
   no health/items and retains no transform. `movements()` provides id/before/
   after/yaw; owner checks unchanged canonical before, builds validated
   EntityTransform, then updates through EntityStore. Owner may refuse intentions.
   `attacks()` provides attacker/target/base_damage3 for vitals owner application.
5. Damage is staged: `prepare_damage(now,victim,raw,context)` does not mutate
   health, loot RNG, dirtiness or emit items. Read the plan's exact `drops().len()`
   and reserve every item/entity/output slot first. Only then `apply_damage(plan)`.
   Refusal owns the intact unpublished plan (`into_plan()`). Successful receipt
   owns its drops and health fact. Retain a receipt on a full downstream queue;
   consume `experience()` before `into_drops()`. Never duplicate from a borrowed
   slice. Plans expire when AI advances beyond their damage tick; if publication
   cannot be reserved, refuse the hit or prepare again against still-current
   health. Never commit zero health before death output capacity is reserved.
6. `dirty_health()` retains <=64 current health facts. Acknowledge through
   `ack_health_saved(fact)` only after that exact admission/version save succeeds.
   Stale generations and removed/reused ids cannot clear newer dirtiness. Removal
   itself does not persist anything. Owner retains final facts as needed.

## Deliberate native policies and limits

Ground walking uses speed attribute *0.5 blocks/tick, with five local candidate
headings, <=original step_height0.6 rise and <=1 block descent. The body must
already be grounded and clear. Swept AABBs close thin/diagonal obstacles, fluids,
hazards and unavailable terrain; no flying, gravity recovery, chunk load or
teleport. Per actor: <=512 cached resident calls, <=8192 collision boxes;
line of sight <=96 half-block samples, querying padded neighboring shape cells.
The same budget closes difficult/long diagonal rays. These are native conservative
motion/sight policies, not original travel/friction/pathfinder parity.

Passives choose eight headings or idle every40 ticks. Zombies choose the nearest
eligible player within original follow_range35, require resident collision-shape
line of sight, and use original unmounted horizontal attack-box inflation
0.8282856485126004 and original20-tick melee cooldown. An obscured nearest target
can block this limited chase even if another farther target is visible. Player
vitals owner handles difficulty, armor, death/respawn and authoritative health.

Ordinary direct mob damage uses exact JVM-checked unenchanted base armor (zombie2,
others0). Normal20-tick hurt immunity allows only increased raw damage in the
first10 ticks, applying the increase. No shields, enchantments, effects,
absorption, equipment, knockback, source-specific bypass tags, plugin events or
Zombie reinforcements. Damage generations/admission/incarnation prevent stale,
foreign, reused-id and repeated-death grants.

Normal adult loot follows pinned JSON ranges: white sheep mutton1..2 +wool1;
cow leather0..2 +beef1..3; pig pork1..3; chicken feather0..2 +chicken1;
zombie rotten_flesh0..2 and ordinary player-kill rare chance1/40 with equal
iron_ingot/carrot/potato choices. Burning cooks meats/potato. XP credited kills:
animals1..3, zombie5. Max2 concrete validated canonical ItemStacks per death.
Native SplitMix64/multiply-high streams are deterministic and isolated for
walking versus loot; keyed vanilla loot RNG sequence parity is not claimed.
No babies, sheep colors/shearing/grazing, breeding/tempting/panic, eggs, riding,
equipment, daylight burn, conversions, doors, despawn, looting/smelts-loot
enchantments or gamerule overrides. Owner must gate drops/XP for gamerules.

## Evidence

Nine focused tests cover executable facts, all8 JVM armor cases, spawn boundary/
light/floor/headroom/hazard/residency facts, walls/fences/open trapdoors/slab steps,
closed unknown/dynamic/unavailable cells, duplicate/tick/cap atomicity, resident
chasing/sight/standing targets20-tick attacks, staged owned death plans/refusals,
exact health dirty acknowledgements and1280 normal loot cases. No full-AI or
full-server differential claim. Standalone harness links existing native rlibs
without declaring this module in the private Sim lib. Exact workspace clippy
flags with -Dwarnings and scoped rustfmt checks are required before handoff.
