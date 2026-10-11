# NET-04.2d — actual native driver and endpoint work order

Root preparation, 11.10.2026. This is the connected follow-up to the accepted
NET-04.2c contract and the SIM-03.4a runtime candidate. It closes the actual
driver/receiver prerequisites before enabling native collector sessions.
This preparation grants no production edits or App activation.

## Starting evidence and ownership

Read the complete NET-04.2c contract and Root review, SIM-03.4a brief, delivery
and eventual runtime review, DUR-04.1/.2 and DUR-03.3 contracts, AGENTS/CLAUDE,
App/Session/Sim invariants, architecture and PAPER_REFERENCE. NET-04.2c brief
SHA256 is `3dff58ac5d1b9511862cf050018fb838ea5f8372b8768fd56a661ab949efd04c`.
SIM-03.4a candidate source manifest SHA256 is
`a47209ad338893f42d62ecd9c3cf4a24f724d5ba63809fbe216729641c7f7c96`.
Its independent runtime review is pending at work-order preparation.
Historical source/evidence freezes remain immutable.

The assigned design agent may write only a new
`docs/agent-tasks/NET-04_2d_NATIVE_DRIVER_ENDPOINT.md` and
`.codex-tmp/net04_2d/**`. Root owns Git, integration and the board. No other
agents, production edits, dependencies or edits to existing contracts.
Use an actual accepted Git archive; exclude concurrent Storage/World work.
Record the exact baseline and source pins before designing the concrete steps.

## Read actual producers before choosing an API

Read complete current startup/service wiring, shard-directory registration,
replacement and unregister/register paths, actual receiver ownership and
consumption, per-connection UUID reservation/quarantine, native ownership
Work/Completion handling, legacy Join/Leave, JoinKit and Stage1/socket flushing,
inventory ownership, chunk acquisition/release and teardown/retry paths.
Follow their real callers and error paths. List the concrete fields and owners
that already bind a receiver to its consuming Sim owner.

NET-04.2c already specifies collector sessions, checked entity/session IDs,
native JoinGame admission, exact registry/mode/pose/hash validation, tracking,
retirement, count/work/scratch limits and terminal disposition. Preserve those
decisions; do not restart their design or add arbitrary proof factories.
The remaining issue is actual control/endpoint production and its integration
order. Prefer a small existing-owner hook where the real implementation suffices.

## Concrete result required

- Specify a real once-bound connection between the registered input endpoint,
  its consumed receiver, the actual native Sim owner and its full generation.
  A stale owner view plus matching shard number, caller bool/hash/label or a
  matching directory lease alone cannot prove replacement receiver ownership.
  Settle initial installation, same-lineage endpoint replacement and fresh
  unregister/register separately. Preserve suspension/refusal when the actual
  receiver/owner connection cannot be established.
- Trace native UUID reservation, provisional network reservation, genuine
  backend read/bootstrap/restore, actual `PlayerAdmitted`, self JoinGame as the
  first router FIFO packet, native spawn projection, remaining Stage1 and chunk
  sends. Bound all retained work and state. Show which step owns each resource
  on every refusal; a failed provisional network reservation cannot erase an
  already admitted native player or pending inventory mutation.
- Trace genuine native Disconnect apply, its borrowed retained detach fact,
  exact original Session retirement, mandatory baseline cleanup and retries,
  late pickup confirmation, full publication FIFO acknowledgement and finally
  zero source pins/consumer refs and both retirement releases. Old connection
  events after rejoin must remain bound to the original admission/session.
- Settle real native input/output dispatch while retaining the current lane
  boundaries. Legacy UUID-only Join/Leave and a `PlayerDetached` intent are not
  native admissions or receipts. No networking mutation of Sim state, raw
  SimShard handed to Session, database work on Sim/async workers, unbounded
  queue, parallel connection inventory or independent legacy block write.
- Give an implementation sequence with one production crate per ticket and
  explicit prerequisites. Keep the Session runtime, App driver, Storage offline
  migration and live gameplay acceptance separate. DUR-03.3a Format6→7 remains
  required before DUR-05 activation; this task cannot silently migrate stores.
- Define meaningful actual-producer tests for installation/replacement,
  foreign/snapshot/stale admission, duplicate UUID/rejoin, uncertainty, bounded
  channels, retained Leave, teardown/quarantine, native Survival JoinGame and
  bit-exact pose/seed, FIFO Stage1 and restart. Distinguish isolated contracts
  from a real client/Paper gameplay comparison. Tests must expose lost work,
  doubled inventory and wrong receiver/session binding, not merely mirror code.

Record relevant full original Minecraft1.21.8/Paper build60 methods and both
patch layers for login, removal, tracking and packet ordering; preserve known
Vanilla/Paper audience differences. A source review is labelled as such.
Finish a concrete contract, exact source/evidence manifest and unresolved
objections for independent Root review. Runtime authorization follows that
review and the actual prerequisite acceptances. Survival phase acceptance and
Java plugin support remain open.
