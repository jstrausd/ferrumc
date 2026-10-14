# Pinned Paper normal-ore subset evidence

Reference: Paper 1.21.8 build 60, checkout commit
`29c8822d90899c89d2689338e81a98f690bcba12`, verified inner JAR SHA-256
`c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`.
`provenance.json` records inspected applied source/patch and output hashes;
`resource-receipt.json` records the accepted GEN06.1 catalog inputs.

Compile each helper with JDK 21 against the verified runtime classpath and run
from a private working directory (Minecraft Bootstrap creates log files):

```sh
javac -cp "$PAPER_CLASSPATH" -d "$HELPER_DIR" ExportOreFacts.java ExportOreRandom.java ExportOreKernel.java
java -cp "$HELPER_DIR:$PAPER_CLASSPATH" ExportOreFacts "$MOTION_OUTPUT"
java -cp "$HELPER_DIR:$PAPER_CLASSPATH" ExportOreRandom "$RANDOM_OUTPUT"
java -cp "$HELPER_DIR:$PAPER_CLASSPATH" ExportOreKernel "$KERNEL_OUTPUT"
```

`ExportOreFacts` exports all 27946 block states' actual blocksMotion facts.
`ExportOreRandom` captures 80 WorldgenRandom chronology/bit cases.
`ExportOreKernel` invokes the full actual OreFeature with ProtoChunks and a
controlled WorldGenLevel proxy: 192 combinations of four substrates, four sizes,
three discard probabilities and four seeds. It captures every changed position
and the final random draw count; no Minecraft server is started.

Run the focused checks from the repository root:

```sh
cargo test --manifest-path server/Cargo.toml --locked --offline -j2 --release -p ferrumc-world --lib survival_ores::tests -- --include-ignored
cargo test --manifest-path server/Cargo.toml --locked --offline -j2 -p ferrumc-world --test generation_contract
```

The ignored smoke test explicitly exercises four adjacent natural seed-0 chunks
and a repeated origin, with all seven ore families present. This revision remains
a selected placed-ore subset with isolated source histories, not Vanilla seed
parity, full FEATURES acceptance, or a running server differential.
