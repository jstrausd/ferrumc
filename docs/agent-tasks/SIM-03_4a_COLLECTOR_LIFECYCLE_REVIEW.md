# SIM-03.4a — Root runtime review

Root review, 11.10.2026. The candidate implements the genuine connected native
player view, original-admission lifecycle cells, applied detach facts and
reserved/retained pickup publication pins. The independent Round2 code review
reports `CLEAN_RUNTIME_REVIEW` with no blocking findings or unresolved objections.
Root acceptance is **CLEAN_RUNTIME** after the final byte audit and completed
independent review receipt. Integration preserves the frozen candidate bytes.

## Exact candidate and check bases

- Whole Sim source:56 files, source manifest SHA256
  `a47209ad338893f42d62ecd9c3cf4a24f724d5ba63809fbe216729641c7f7c96`.
- Author evidence:40 files, evidence manifest SHA256
  `9cfed09991ce6d6e71108896040cfbc059abbc662837e3f8536822fd45e9e1eb`.
- [Author delivery](SIM-03_4a_COLLECTOR_LIFECYCLE_DELIVERY.md) SHA256
  `3713faf9c6b5b0afa67c4f1e2a308a9b442105abb97a663d11e063a4f1502d19`.
- Author checks use accepted `d3fca66c`; Root whole checks and independent
  probes use the actual accepted `e772564c06445b9309a9294460c8988e7269916a`
  archive plus the entire frozen Sim candidate and its delivery.
- Root archive:1267 files;1215 outside files unchanged. All234 accepted
  GEN-05.7 source files remain byte-identical. Shared Storage work was excluded.
- Published `80c3221c` changes only the two Root boards and the new NET-04.2d
  work order. Root checked that exact three-path delta; production,
  dependencies and the candidate are unchanged from the checked basis.

The first independent review was interrupted by a provider limit and supplied
no acceptance. Its three artifacts remain unchanged. Round2 performs its own
complete review and does not infer code correctness from successful tests.
Its final report SHA256 is
`618d52b6a23afc404161685394d796526d4cb9eb69a1ec23daea032ff93aee31`;
assessment SHA256 is
`2b5921d1e3344bd92defb0e7428cb3663567fbe7335b6fe70a0f735eeffdbcd5`.

## Runtime and authority review

Root read both new lifecycle production modules completely and the integration
diffs, checked the connected-view producer against actual ownership/admission
checks, and inspected the independent probes and their actual completion paths.
The independent reviewer read all15 changed/new files in full, existing
restore/candidate/confirm/release/rejoin/disconnect/FIFO/countershadow paths,
related evidence/codec/checkpoint/inventory/entity production, and the actual
scheduler quiescence and Stop call graph.

The view borrows actual native state and validates owner, admission, full
current scope/generation, connected state and Stop. Native mode/pose/look come
from that state; the seed hash uses the existing World producer. Drain keeps
the output authority of an actually connected admission.

Lifecycle storage is checked actual Playerlimit + Publicationgroup limit,
with a positive at-most-one-MiB scalar layout cap and fallible reservation.
Bootstrap/restore admission checks precede native body copies. Bootstrap
uncertainty retains its reserved cell, definitive abort releases it, and only
actual admitted handles seal cells. Rejoin always receives a new cell.

Detach facts are minted only by the real native Disconnect apply. Repeated
historical disconnects retain their original fact and cannot mutate a new
admission. Each actual animated pickup holds exactly one original cell pin;
pending install reserves it, confirmation retains it, and definitive abort or
valid whole-publication FIFO acknowledgement releases it once. Busy,
uncertainty, refresh, quarantine and detach keep it. The actual countershadow
uses the same transitions over its genuine completion prefix and future
admission state. Unresolved cells prevent Stop and scheduler quiescence.

Historical tickets require applied detach and zero source pins. Acknowledgement
revalidates slot/credit identity, original collector, full generation and zero
pins. Stale, foreign, repeated and reused-slot tickets cannot release credits.
Snapshot clones contain no runnable lifecycle authority.

## Verification

Root whole-Sim Debug and Release each pass369 tests with no failed, ignored or
filtered tests. Workspace/all-target Clippy with `-D warnings`, Workspace fmt
and Sim rustdoc with `-D warnings` pass. All five command records and log hashes
were freshly verified. These checks already contain accepted GEN-05.7 and
accepted Storage production; no author WIP is part of their dependency basis.

The independent reviewer added two private probes, each using genuine Memory
and redb admission, backend Work execution and actual Completions:

1. Completion, Disconnect and Rejoin in one boundary despite the Completion's
   later queue position: original pin retained, new admission pinless,
   conserving inventory, FIFO/lifecycle acknowledgement and stale events after
   slot reuse without retargeting.
2. Three historical admissions with separate confirmed pickup pins:
   reordered acknowledgement refuses, FIFO acknowledgements release only
   original cells, stale tickets refuse, new player stays connected, and final
   native Disconnect/acknowledgement/Stop succeeds.

Both probes pass, four backend scenarios in total. They modify only a private
test import and new private test file; they are absent from the delivered
production tree. The independent log SHA256 is
`48d645e7d76ae336a4e5caef6db12becbf3c670498a7d01a1ada97e6e1145229`.

Root freshly verified56 source files,40 evidence files,10 input pins,8 producer
pins,25 NET reference pins,9 author direct-read pins, delivery, accepted archive
and checked overlay. The independent final verification agrees, including the
exact two private probe mutations and preserved interrupted-review artifacts.
Records are under `.codex-tmp/sim03_4a-root-review/` and
`.codex-tmp/sim03_4a-independent-review-round2/`.

The reviewer read full relevant original Take/removal/broadcast methods and
their forwarding callees, the Take source patch, tracker-desync feature patch
and seed producer. Applied Paper broadcasts Take through collector tracking
including self; Vanilla uses the item tracker. This is source review, not a
running Paper gameplay comparison.

## Consumer boundary

Source-pins-zero in Sim does not prove original Session retirement or zero
consumer references. Before lifecycle acknowledgement the actual driver must
prove those separately; before publication acknowledgement it must complete
the entire ordinary or terminal group admission. Tickets are not socket
receipts. The accepted contract leaves those explicit responsibilities to
Session and driver follow-ups, including NET-04.2d.

This Sim candidate changes no live App join, connection inventory, world
generator or Java event consumer. Live item physics, Survival, Nether/End/Dragon,
full plugin support and equivalent-scope resource measurements remain open.
