# Original Minecraft 1.21.8 biome seed goldens

`biome_seed_goldens.tsv` records numeric input and output of the actual original
server method. `BiomeSeedGolden.java` calls `dnz.a(long)` by reflection; it
contains no implementation of the hash. The pinned Mojang mappings map that
class/method to `net.minecraft.world.level.biome.BiomeManager.obfuscateSeed(long)`.
The default package cannot contain the unsigned runner because Mojang signs
its default-package classes, so the runner lives in `fixture`.

Oracle generated with Java 21.0.12.1 from these original artifacts:

- Bundled original server: SHA256 `2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7`.
- Extracted server-1.21.8.jar: SHA256 `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`.
- Original server mappings: SHA256 `e373ba569839e6f3c7bd4880247f3e85577d92cde2c605adfa2024eb9cae503e`.
- Guava: original distribution's `guava-33.3.1-jre.jar`, SHA256 `4bf0e2c5af8e4525c96e8fde17a4f7307f97f8478f11c4c8e35a0e3298ae4e90`.

Compile the runner with `javac -d <output> BiomeSeedGolden.java`, then invoke
`fixture.BiomeSeedGolden` with that directory, the original extracted server
and the original Guava library on the classpath. The Rust unit test consumes
the recorded oracle outputs at 0, ±1, signed64 extrema, an endian-sensitive
positive value, and ±123456789. This is an original-method differential
comparison, not a live Paper world/restart comparison.
