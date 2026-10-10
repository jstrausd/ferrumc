# Adopted original multinoise census

`data/biome-parameters-1.21.8.json` is unchanged from the independently frozen
normal and optimized Root census. All ten original freeze files were checked
before adoption. The unchanged helper, historical runner, two identical manifests
and independent freeze are retained here as origin evidence. Compiled classes and
duplicated megabyte output copies are not vendored; their historical pins remain
in the freeze. There is one production copy of the exact parameter bytes.

The Java helper actually invokes the original
`MultiNoiseBiomeSourceParameterList.knownPresets()` and `.values()` after the
original bootstrap. It preserves every post-quantization long and list position,
without copied Overworld formulas. The original export checked 19 server class
CodeSources and its freshly compiled helper origin. The manifests record fixed
input receipts for that historical execution. The runner is preserved unchanged
as evidence, not relocated or advertised as a current portable rerun command.
Its dynamic input hashing and three-variable Java environment scrub are not
claimed to satisfy later strengthened rerun guards.

Run `python3 tests/reference/verify_biome_parameters.py` from the crate, and repeat
under `python3 -O`. The offline verifier checks all seven fixed production/origin
file pins, all 7598 rows against the full 65-key original export, order, references,
integer domains, historical origin receipts and the independent normal/-O freeze.
The accompanying unittest supplies explicit pin/schema negative guards in either
Python mode. `verify_builtin_biomes.py` independently verifies all 65+2 real
original JAR resources, datagen raw equality, Decimal content and source receipts.

This is static builtin resource-data acceptance. It does not execute or validate
dynamic biome registry IDs, NETWORK_CODEC payloads, selection/RTree history,
surface rules, active worlds, survival or plugin compatibility.
