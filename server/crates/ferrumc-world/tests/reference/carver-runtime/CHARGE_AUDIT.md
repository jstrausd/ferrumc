# Runtime charge correspondence

This records the production correspondence to corrected-contract §5 and §6.
It is a structural audit, not a new numerical oracle. The accepted recurrences
and public/native caps are unchanged. Exact-fault adapters add no charges.
Private and public paths execute the same sampling and constructor cores.

| Production path | Charged operation and bound |
| --- | --- |
| `geometry::execute` | Copy 98304 states,1536 fluid words,256 heights; reset1536 mask words; Empty/Published2:101634. Source entry289. Starttest867. |
| `Engine::{tunnel,canyon,ellipsoid}` | One per tunnel/Canyon step, width iteration, ellipsoid, column test and Y iteration. Original G=63831207759 covers count112, child thickness≤1 with depth2,197 total tunnel steps and10990 Cave ellipsoids. |
| `Engine::{block,write}` | Block, Aquifer call, flag read, fix-up, each write and each rescan row charged before operation. Mask bounds96000 block entries; original B=74304000 includes two writes and two possible rescans per entry. |
| `DirectCarverBiomes::biome` | Exactly74 before the lazy lookup:64 LCG steps,8 candidates,memo,entry.≤96000 consuming fix-ups plus289 source entries give7104289. R=63912717682. |
| `Cold::{visit,intern}`, Keys | Existing Cold entry, equality hash/word, collision, identity lookup and publish charges now use `core_charge`; same admission and loops. V131072,K4096,key length70 give117744161352 total constructor bound with all rows below. |
| Normal/Improved/Perlin/Blended basis cores | Existing Paper basis constructors meter the same260-per-octave bounded operations;64 slots,2 Normal components and40 Blended slots. Copied Normalbasis4096·2·64·260,Blended4096·40·260. Meter admitted before first charge. |
| Cold spline plan/build/bounds | Shared charge paths for pending entries, points, coordinate lookup, lists/specs and bounds refresh. Per K row16385+1024·64+16384+64+1024+16384+1024; refreshV·64. |
| Chunk graph/cache/scratch construction | Existing validator, edge, root, wrapper and scratch charges2K+8192+22+128; Flatcache fill128·25·(3706+1); empty Blender50. No new wrapper or constructor allocation policy. |
| Density/Marker/Spline/Noise sampling cores | Density entry1; alias0; binary both children; range input+max branch; Normal sampler plus octave slots; shared spline entry/coordinates/search/≤2 adjacent children. Foreign marker delegates at most child. Root bounds4,4,4,4,39,39,46,38,564,40,1108,3706,7,18,4; climate39,39,46,38,564,40 plus6 quantization charges≤772. |
| Carver climate | Existing direct SinglePoint sampling with private checked address.≤96289 genuine queries,74335108 charges. Public coordinate/cap constructors unchanged. |
| MaterialBlocks / Aquifer construction | Material resolution≤16384; Aquifer1+2·315=631. Prepare reserved117744178367; actual used is checked before Ready. |
| Aquifer hit/pressure | Existing entry/global,12 candidate loop+lookup charges,status lookup and≤3 pressure entries.≤40 plus one≤4 barrierroot per invocation:4224000. Locations/statuses never evict. |
| Aquifer location miss | Factory and publication2 each of≤315 misses:630. Three original bounded calls per location; every Xoroshiro draw charged.≤945P,P=2^128−1. Legacy geometry rejection draws remain explicitly C11 unmetered. |
| Aquifer status miss | Existing status compute/global/offset/global/surface/type/sample entry paths≤64+38+564+4+4+4=678 per≤315 misses:213570. |
| Preliminary cache |≤315·13 requests,≤297 comparisons each:1216215. Each of≤297 one-time misses scans≤49 heights with entry+root1108,then publish:16139574. Cache2048 entries and original no eviction remain intact. |
| Selector shared search core | Entry/publish2,node visits9112,prior branch distance8·1519,edge work≤19·9111:194375/query;96289 queries18716174375. Treebuild retains its separate public budget. |
| Shared Surface rule evaluator | Per fix-up begin-column2,block entry1,closed Overworld evaluator467:96000·470=45120000 under unchanged45908736. Direct source called only at existing biome memo miss; temperature shares memo. |

Noise total321566836740286847972889004023138800132439 includes prepare.
Carve checks remaining321566836740286847972889004023021055954072;
it never reserves the full total twice. Actual constructor and run reports
are separate measured values and do not replace these structural bounds.

`CarverCumulativeMeter` owns limit/used in three checked u64 limbs. Charge
checks overflow and limit before changing any field, then only refills a
bounded segment counter. It owns no producer history. Sampling adapters keep
`ChargeFault::Carver` intact through basis/RNG/Cold/Keys/Spline/density/state/
climate/preliminary/Aquifer/selector and the lazy rule provider. Public fault
adapters remain confined to publicly constructed meters. Full-carry tests
compare entire caches, epochs, RNG projections, selector leaf and Aquifer flag.

The wide period proof is finite accounting, not a time or performance bound.
Earlier immutable-router RNG shuffles and C11 geometry rejections retain their
accepted scopes. Static SIN262144B is immutable and separately counted.
