# Original placement collision fixture

Reference: Paper 1.21.8 Build 60, commit
`29c8822d90899c89d2689338e81a98f690bcba12`, protocol 772. The inner jar SHA-256
is `c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`.
`provenance.json` records source/TSV/registry hashes and explicit scope.

The oracle runs original `CollisionContext.placementContext(null)`, state
collision shape queries, `EntityDimensions.scalable(0.6F,1.8F).makeBoundingBox`
and `Shapes.joinIsNotEmpty(..., BooleanOp.AND)`. Source reviewed:
BlockItem.canPlace, Level.checkEntityCollision, EntityGetter.isUnobstructed,
Player.STANDING_DIMENSIONS, EntityDimensions and the context-specific shape
implementations. Shape export includes placement and empty contexts, and a
shifted-position query. Genuine geometric goldens include eight body positions
at each of block origins `(0,0,0)` and `(10,64,-7)`, with epsilon controls.

Use JDK 21 and a verified classpath of the pinned inner jar plus its original
runtime libraries. From the repository root, with task-specific shell variables
pointing to that JDK, classpath text, inner jar and an output directory:

```sh
mkdir -p "$collision_output/classes"
"$collision_jdk/javac" -proc:none -cp "$(cat "$collision_classpath")" \
  -d "$collision_output/classes" \
  server/crates/ferrumc-placement/data/collision/CollisionShapeOracle.java
"$collision_jdk/java" -cp "$collision_output/classes:$(cat "$collision_classpath")" \
  CollisionShapeOracle "$collision_inner" "$collision_output/shapes.tsv"
python3 server/crates/ferrumc-placement/data/collision/generate.py \
  "$collision_output/shapes.tsv"
rustfmt --edition 2021 server/crates/ferrumc-placement/src/collision_data.rs
```

Generation rejects any mismatch in all state IDs, names and properties against
registry blocks.json. It produces the state-to-shape binary, generated Rust box
table, body controls and genuine probe binary. Production only embeds the shape
index/table; the probe binary is test-only. The crate's differential test checks
446,432 supported original cases. Offset bamboo/dripstone and moving piston (44
states) refuse rather than silently using a wrong cube. Fresh closed shulkers
and placement-empty scaffolding/powder snow are supported. These checks do not
establish world support, non-player entity, alternate pose, canSee or plugin
event parity.
