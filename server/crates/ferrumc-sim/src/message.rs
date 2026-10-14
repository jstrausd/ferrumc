//! Messages crossing the simulation boundary.
//!
//! [`GameInput`] flows *into* a shard — upstream the session router produces it
//! from validated network events — and is applied only at tick boundaries.
//! [`GameOutput`] flows *out* of a shard after a tick and is routed back to
//! sessions/network. The simulation never touches sockets; it only exchanges
//! these typed messages.

use ferrumc_core::{GameMode, PlayerId};
use ferrumc_math::{BlockPos, Cuboid, Direction, Vec3};
use ferrumc_world::{BlockStateId, Sign, SIGN_LINES};

use crate::mutation::MutationCause;
use crate::region::RegionOp;

/// An input applied to the simulation at the next tick boundary.
///
/// Inputs are intentionally minimal for this milestone: player presence and
/// movement. The enum is `#[non_exhaustive]` because new variants will be added
/// as the simulation grows, so downstream `match`es must include a wildcard arm.
///
/// `PartialEq` (but not `Eq`) is derived because positions are floating-point
/// [`Vec3`]s.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GameInput {
    /// A player entered the shard.
    PlayerJoin {
        /// Identity of the joining player.
        player: PlayerId,
        /// World-space position the player joins at.
        position: Vec3,
    },
    /// A player moved, rotated, or both.
    ///
    /// Carries each component independently so one variant covers all three
    /// serverbound move packets: a position-only `SetPlayerPosition`
    /// (`position` set, `yaw`/`pitch` `None`), a `SetPlayerPositionAndRotation`
    /// (all set), and a rotation-only `SetPlayerRotation` (`position` `None`,
    /// `yaw`/`pitch` set). A `None` field leaves that component unchanged; an
    /// input with no component set has no effect. `yaw`/`pitch` are degrees.
    PlayerMove {
        /// Identity of the moving player.
        player: PlayerId,
        /// New world-space position, if this move changed it.
        position: Option<Vec3>,
        /// New body yaw in degrees, if this move changed it.
        yaw: Option<f32>,
        /// New pitch in degrees, if this move changed it.
        pitch: Option<f32>,
    },
    /// A player left the shard.
    PlayerLeave {
        /// Identity of the leaving player.
        player: PlayerId,
    },
    /// Restores checked legacy player health/hunger and every saved timer.
    RestoreVitals {
        /// Actual joining legacy actor.
        player: PlayerId,
        /// Complete validated state, never a partial HUD projection.
        snapshot: crate::VitalsSnapshot,
    },
    /// Applies one pinned App consumption plan; commit inventory only after its result.
    FoodConsumed {
        /// Eating actor.
        player: PlayerId,
        /// Trusted resolved food properties.
        food: ferrumc_items::FoodProperties,
        /// Positive increasing App token; a replay never grants food again.
        token: u64,
    },
    /// Applies trusted source/armor facts to the actual player health owner.
    PlayerDamage {
        /// Victim.
        player: PlayerId,
        /// Checked finite source policy and ordinary armor.
        request: crate::DamageRequest,
    },
    /// Attacks a player within ordinary three-block contact reach.
    PlayerAttack {
        /// Living attacker.
        player: PlayerId,
        /// Present target, resolved from the actual wire-id registration.
        target: PlayerId,
        /// Trusted held-tool/armor source facts, never client damage numbers.
        request: crate::DamageRequest,
    },
    /// Ordinary strength-scaled attack with a committed-damage receipt.
    PlayerAttackTracked {
        /// Genuine living attacker, whose elapsed strength is owner state.
        player: PlayerId,
        /// Present target resolved from its genuine player wire registration.
        target: PlayerId,
        /// Trusted unscaled held damage and current victim armor policy.
        request: crate::DamageRequest,
        /// Trusted effective ordinary attack-speed attribute, finite 0.1..=1024.
        attack_speed: f32,
        /// Positive increasing exact App weapon/armor witness token.
        token: u64,
    },
    /// Attacks a genuine canonical mob using actual elapsed player strength.
    PlayerMobAttackTracked {
        /// Living genuine attacker holding the exact App weapon witness.
        player: PlayerId,
        /// Actual nonplayer wire registration, never a guessed numeric ID.
        target: ferrumc_core::EntityId,
        /// Trusted ordinary unscaled weapon damage.
        damage: f32,
        /// Trusted finite ordinary attack-speed attribute.
        attack_speed: f32,
        /// Positive increasing exact App witness token, shared with tracked player combat.
        token: u64,
    },
    /// Motion facts admitted alongside the actual movement input.
    FallMovement {
        /// Living actor.
        player: PlayerId,
        /// Exact finite admitted packet position.
        position: Vec3,
        /// Actual protocol ground flag, also checked against coarse block support.
        on_ground: bool,
        /// Actual current `PlayerInput` sprint flag.
        sprinting: bool,
    },
    /// Requests dead-player respawn at the App-selected safe spawn.
    Respawn {
        /// Dead actor.
        player: PlayerId,
        /// Trusted safe spawn coordinate.
        position: Vec3,
    },
    /// Applies an exact pinned App bucket inventory plan against current water.
    BucketUse {
        /// Present living actor.
        player: PlayerId,
        /// Trusted server-selected source or pour target.
        position: BlockPos,
        /// Client action sequence for block acknowledgement.
        sequence: i32,
        /// Empty-bucket source pickup when true, water-bucket pour when false.
        fill: bool,
        /// Positive increasing exact App inventory-plan witness.
        token: u64,
    },
    /// Explicitly wakes a resident natural/loaded water cell without force loading.
    WaterSchedule {
        /// Exact resident water-containing coordinate.
        position: BlockPos,
    },
    /// A player broke (destroyed) the block at `position`.
    ///
    /// Decoded upstream from a serverbound `PlayerAction` "start destroying
    /// block". The simulation validates the edit at the tick boundary
    /// (actor present, target chunk resident, target within reach) and, on
    /// acceptance, replaces the block with [`BlockStateId::AIR`]. Held-tool and
    /// drop rules are out of scope this milestone.
    BlockBreak {
        /// Identity of the player breaking the block.
        player: PlayerId,
        /// Absolute position of the block to break.
        position: BlockPos,
        /// The block-action sequence the client stamped on the originating
        /// `PlayerAction`, echoed back in an `AcknowledgeBlockChange` on accept.
        sequence: i32,
    },
    /// Starts ordinary Survival mining with the application's authoritative tool.
    MiningStart {
        /// Actor whose mode and reach are checked at the boundary.
        player: PlayerId,
        /// Absolute target cell.
        position: BlockPos,
        /// Client prediction sequence.
        sequence: i32,
        /// Held item and environment facts, never client-authored slot bytes.
        tool: crate::MiningTool,
    },
    /// Finishes a started mining action, or schedules its remaining tick work.
    MiningFinish {
        /// Mining actor.
        player: PlayerId,
        /// Target must match the active block and its unchanged preimage.
        position: BlockPos,
        /// Client prediction sequence.
        sequence: i32,
        /// Current authoritative tool; changed tools cancel the old action.
        tool: crate::MiningTool,
    },
    /// Cancels mining without changing any block or producing drops.
    MiningAbort {
        /// Actor whose mining work is cancelled.
        player: PlayerId,
        /// Client's target, also cleared when it differs from active work.
        position: BlockPos,
        /// Prediction sequence to acknowledge.
        sequence: i32,
    },
    /// Invalidates active mining after any exact held-slot or selection mutation.
    MiningToolChanged {
        /// Actor whose held stack or selection changed.
        player: PlayerId,
        /// Authoritative current context.
        tool: crate::MiningTool,
    },
    /// Refreshes the accepted ground flag without cancelling accumulated mining.
    /// Eye-water state is read from the simulation's current resident world.
    MiningEnvironmentChanged {
        /// Actor whose accepted movement supplied the new ground flag.
        player: PlayerId,
        /// Current accepted serverbound ground flag.
        on_ground: bool,
    },
    /// A player placed a block at `position`.
    ///
    /// Decoded upstream from a serverbound `UseItemOn`; `position` is the block
    /// selected by the host from the authoritative clicked/adjacent cells. The
    /// simulation validates the edit at the
    /// tick boundary (the same checks as [`BlockBreak`](GameInput::BlockBreak))
    /// and, on acceptance, computes the *correct* placed block-state from
    /// `state` (the held item's default), `clicked_face`, `cursor_position`, and
    /// `player_yaw` via `ferrumc_placement::compute_placement` (rotation/facing/
    /// half/fence connectivity) before writing it through the single block-edit
    /// funnel. An unsupported or unrecognised block falls back to writing `state`
    /// unchanged. `state` itself is resolved by the app from the player's held
    /// hotbar stack, so the simulation stays inventory-free.
    BlockPlace {
        /// Identity of the player placing the block.
        player: PlayerId,
        /// Absolute position the block is placed at.
        position: BlockPos,
        /// The block-action sequence the client stamped on the originating
        /// `UseItemOn`, echoed back in an `AcknowledgeBlockChange` on accept.
        sequence: i32,
        /// The held item's default block-state (the placement input); the
        /// simulation refines it into the final placed state.
        state: BlockStateId,
        /// The face of the targeted block the player clicked, used to derive axis
        /// (logs), half (slabs/stairs), and wall-torch facing.
        clicked_face: Direction,
        /// The cursor hit point inside the targeted block (`0.0..=1.0` per axis);
        /// its `y` selects a slab/stair half.
        cursor_position: Vec3,
        /// The player's yaw in degrees at place time, used to derive horizontal
        /// facing (stairs face the player; furnaces face away).
        player_yaw: f32,
    },
    /// Write an authoritative, *exact* block-state at `position`, applied
    /// verbatim — **not** refined by `compute_placement`.
    ///
    /// Produced by a plugin/command exact-state write (a `before_block_*`
    /// `Replace` decision or a `WorldIntent::SetBlock`): the state is already the
    /// final one the plugin chose, so the simulation must store it byte-for-byte
    /// rather than re-deriving axis/half/facing from neutral placement inputs
    /// (which would, for example, rewrite a plugin-set `oak_log axis=x` to the
    /// default vertical `axis=y`). Only a player [`UseItemOn`](GameInput::BlockPlace)
    /// placement is refined by `compute_placement`.
    ///
    /// It is still validated by the same block-edit funnel as every other edit
    /// (actor present, target chunk resident, target within reach); on acceptance
    /// it emits a [`GameOutput::BlockChanged`] carrying the acting player so the
    /// actor still gets the ack/resync. This ordinary world-intent variant never
    /// emits a placement completion, even when client sequences repeat.
    /// Player-attributed exact states must also
    /// pass final-state player-body obstruction; spectators cannot apply them.
    /// No placement refinement or fence-neighbour pass runs.
    SetBlockExact {
        /// Identity of the player on whose behalf the exact write is applied (the
        /// actor that receives the ack/resync).
        player: PlayerId,
        /// Absolute position the block is written at.
        position: BlockPos,
        /// The block-action sequence to echo back in an `AcknowledgeBlockChange`
        /// on accept.
        sequence: i32,
        /// The exact block-state to write, applied verbatim.
        state: BlockStateId,
    },
    /// Explicit player placement replaced by a plugin-selected exact state.
    ///
    /// Applies the same exact-state validation and collision gate as
    /// [`SetBlockExact`](Self::SetBlockExact), with one opt-in
    /// [`PlayerPlacementResolved`](GameOutput::PlayerPlacementResolved) result.
    /// Ordinary exact world intents must use `SetBlockExact` so a reused client
    /// sequence cannot prematurely complete a different placement action.
    SetBlockExactTracked {
        /// Actor receiving the correction and acknowledgement.
        player: PlayerId,
        /// Absolute final primary cell.
        position: BlockPos,
        /// Original explicit placement action sequence.
        sequence: i32,
        /// Plugin-selected final exact state.
        state: BlockStateId,
    },
    /// Request a resync + acknowledgement for a block edit that was refused
    /// *upstream* of the simulation (a plugin `Deny`, spawn-protection veto, etc.)
    /// without ever mutating the world.
    ///
    /// The upstream layers (net/app) have no world access, so they cannot read the
    /// authoritative block state to heal the actor's optimistic prediction. They
    /// route the refusal here instead: at the tick boundary the shard reads the
    /// authoritative state at `position` from the resident chunk and emits a
    /// [`GameOutput::BlockChangeRejected`] — the same ack + mandatory resync funnel
    /// an in-sim rejection (out of reach, unloaded chunk) already uses — so every
    /// rejection site heals the client identically. No chunk write and no journal
    /// entry are produced; the handler only *reads* chunk state, keeping the tick
    /// deterministic.
    RejectBlockEdit {
        /// Identity of the player whose predicted edit must be undone.
        player: PlayerId,
        /// Absolute position of the refused edit (where the authoritative state is
        /// read to heal the client).
        position: BlockPos,
        /// The block-action sequence the client stamped on the originating edit,
        /// echoed back in an `AcknowledgeBlockChange` so its pending prediction
        /// ends.
        sequence: i32,
        /// The state the client optimistically predicted (air for a refused break,
        /// the held block for a refused place); used only to classify the metric,
        /// never applied.
        requested_state: BlockStateId,
    },
    /// Set the authoritative server-side game mode of a player.
    ///
    /// Produced by the app's `/gamemode` command path (in addition to the
    /// clientbound `GameEvent` that switches the client visually) so the
    /// simulation owns the mode that later enforcement reads (creative
    /// no-decrement, block-break speed, flight). Applied at the tick boundary;
    /// a `SetGameMode` for an absent player is a silent no-op. Emits no output.
    SetGameMode {
        /// Identity of the player whose mode changes.
        player: PlayerId,
        /// The new authoritative game mode.
        mode: GameMode,
    },
    /// Apply a region (cuboid) block edit on behalf of `player`.
    ///
    /// Produced by the app's `/fill` and `/replace` commands. The whole cuboid is
    /// applied at one tick boundary through the same single block-edit funnel a
    /// single edit uses (under [`MutationCause::Command`]): every changed cell
    /// persists via the chunk overlay and broadcasts a `BlockUpdate` to viewers,
    /// but carries no acked sequence (a command edit is authoritative, not a
    /// client prediction). The shard captures the prior block-state of every
    /// changed cell into a bounded per-player undo history before applying, so a
    /// later [`RegionUndo`](GameInput::RegionUndo) can restore it.
    ///
    /// The edit is bounded: a cuboid whose volume exceeds the shard's configured
    /// region cap is rejected wholesale (the command layer already rejected it
    /// with a user-facing error; this is defense in depth). Cells in chunks not
    /// resident in this shard are skipped (cross-shard region edits are out of
    /// scope this milestone).
    RegionEdit {
        /// Identity of the player on whose behalf the edit is applied (keys the
        /// undo history; the player need not be present, since a command edit is
        /// authoritative and not reach-checked).
        player: PlayerId,
        /// The cuboid of blocks the edit addresses.
        region: Cuboid,
        /// How every block in the cuboid changes.
        op: RegionOp,
    },
    /// Undo `player`'s most recent region edit, restoring the prior block-states
    /// it captured.
    ///
    /// Produced by the app's `/undo` command. Pops the player's newest undo entry
    /// and re-applies each captured prior state through the same funnel (so the
    /// restoration also persists and broadcasts). An undo is *not* itself pushed
    /// onto the history, so repeated `/undo` walks back through successive edits.
    /// A `RegionUndo` for a player with no history is a silent no-op.
    RegionUndo {
        /// Identity of the player whose most recent region edit is undone.
        player: PlayerId,
    },
    /// A player submitted new text for one face of a sign they were editing.
    ///
    /// Decoded upstream from a serverbound `UpdateSign`. The simulation validates
    /// the edit at the tick boundary (the actor is present, the target chunk is
    /// resident, the target is within [`MAX_REACH`](crate::SimShard) reach, and a
    /// sign block-entity exists there and is not waxed) and, on acceptance,
    /// replaces the addressed face's four text lines and emits a
    /// [`GameOutput::SignUpdated`] for the session layer to broadcast as a
    /// `BlockEntityData`. A failed validation is a silent no-op (no output, no
    /// mutation): net never writes the world directly, so a stale or out-of-reach
    /// edit is simply dropped.
    UpdateSign {
        /// Identity of the editing player.
        player: PlayerId,
        /// Absolute position of the sign being edited.
        position: BlockPos,
        /// `true` to edit the front face, `false` the back.
        is_front: bool,
        /// The four new text lines, top to bottom (each a plain string the
        /// network layer wraps as a text component).
        lines: [String; SIGN_LINES],
    },
}

/// An output produced by the simulation during a tick.
///
/// Outputs are deterministic given the inbox contents: identical input
/// sequences yield identical output sequences. The enum is `#[non_exhaustive]`
/// for the same forward-compatibility reasons as [`GameInput`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GameOutput {
    /// Always resolves a tracked mob hit, accepted only after actual health commit.
    MobAttackResolved {
        /// Genuine attacker holding the weapon witness.
        player: PlayerId,
        /// Actual canonical target wire registration.
        target: ferrumc_core::EntityId,
        /// Original App witness token, also returned on refusal.
        token: u64,
        /// Positive committed health damage; all death drops were reserved first.
        accepted: bool,
        /// Actual controller health damage, zero on refusal.
        damage: f32,
        /// Actual credited player-kill experience after reserved death, otherwise zero.
        experience: u8,
    },
    /// Actual AI melee damaged player health using the current trusted armor profile.
    MobPlayerHurt {
        /// Genuine victim.
        player: PlayerId,
        /// Genuine committed hostile-mob wire registration.
        attacker: ferrumc_core::EntityId,
        /// Actual positive health damage.
        damage: f32,
        /// Incoming incremental damage before ordinary armor, for pinned wear.
        raw_damage: f32,
    },
    /// Resolves one exact combat witness from actual committed player health.
    PlayerAttackResolved {
        /// Attacker holding the App witness.
        player: PlayerId,
        /// Target whose health was actually checked.
        target: PlayerId,
        /// Original positive App witness token, also returned on refusal.
        token: u64,
        /// True only when actual health decreased by positive damage.
        accepted: bool,
        /// Actual committed health damage, zero on refusal.
        damage: f32,
        /// Strength-scaled incoming damage after hurt immunity, before armor;
        /// zero on refusal. App uses the pinned armor witness for item wear.
        raw_damage: f32,
    },
    /// A loaded ordinary-furnace tick or discovery retained its complete data.
    FurnaceTickDeferred {
        /// Resident position whose tick could not commit.
        position: BlockPos,
        /// Classified owner/planner admission refusal.
        reason: crate::FurnaceError,
    },
    /// Terminal bucket world result; only Some permits the pinned inventory commit.
    BucketUseResolved {
        /// Original bucket actor.
        player: PlayerId,
        /// Original target.
        position: BlockPos,
        /// Original block acknowledgement sequence.
        sequence: i32,
        /// Original pickup/pour direction.
        fill: bool,
        /// Exact positive App plan token.
        token: u64,
        /// Actual committed block postimage; None preserves prepared inventory.
        state: Option<BlockStateId>,
    },
    /// A water tick or explicit schedule retained/refused work before world mutation.
    WaterTickDeferred {
        /// Original due cell, retained for retry when applicable.
        position: BlockPos,
        /// Classified bounded refusal; unsupported rules never guess a replacement.
        reason: crate::WaterError,
    },
    /// Terminal uncommitted mining witness; App retires the exact held snapshot.
    MiningReleased {
        /// Original mining actor.
        player: PlayerId,
        /// Positive immutable App witness; zero-context callers emit no release.
        tool_token: u64,
    },
    /// Actual current owner health/hunger; Session publishes `UpdateHealth`.
    PlayerVitalsChanged {
        /// Actor whose vitals changed.
        player: PlayerId,
        /// Complete owner state; packet consumers use its visible fields.
        snapshot: crate::VitalsSnapshot,
    },
    /// Complete final owner state before an actual legacy actor is removed.
    /// This save observation has no client health-packet interpretation: silent
    /// exhaustion/timer changes must survive same-boundary departure too.
    PlayerDepartureVitals {
        /// The departing actual player.
        player: PlayerId,
        /// All nine health, hunger and timer fields at removal.
        snapshot: crate::VitalsSnapshot,
    },
    /// Single living-to-dead edge; App owns death-screen and inventory policy.
    PlayerDied {
        /// Dead actor.
        player: PlayerId,
    },
    /// Effective damage after cooldown and ordinary armor.
    PlayerHurt {
        /// Victim.
        player: PlayerId,
        /// Actual health loss, excluding overkill.
        amount: f32,
    },
    /// Result of the exact pinned App inventory-consumption plan.
    FoodConsumptionResolved {
        /// Original actor.
        player: PlayerId,
        /// Original monotonic request token.
        token: u64,
        /// Whether the owner applied food; only true permits inventory commit.
        accepted: bool,
    },
    /// Owner accepted a dead-player respawn and reset health/hunger/timers.
    PlayerRespawned {
        /// Revived actor.
        player: PlayerId,
        /// Actual new owner position.
        position: Vec3,
    },

    /// A player became present in the shard.
    PlayerSpawned {
        /// Identity of the spawned player.
        player: PlayerId,
        /// World-space position the player spawned at.
        position: Vec3,
    },
    /// A player's position and/or rotation changed.
    ///
    /// Always carries the player's current absolute `position`, `yaw`, and
    /// `pitch` (degrees) as of this tick, plus `position_changed`: `true` when
    /// this output applied a new position (the session layer broadcasts a
    /// relative/absolute move carrier), `false` for a rotation-only change (the
    /// session layer broadcasts a rotation-only carrier). The head yaw is sent
    /// alongside either way so a viewer's head turns.
    PlayerMoved {
        /// Identity of the moved player.
        player: PlayerId,
        /// Current absolute world-space position.
        position: Vec3,
        /// Current body yaw in degrees.
        yaw: f32,
        /// Current pitch in degrees.
        pitch: f32,
        /// `true` if this tick applied a new position; `false` if it was a
        /// rotation-only change.
        position_changed: bool,
    },
    /// A player's movement was rejected; the client should snap back to this
    /// authoritative position.
    ///
    /// Emitted at the tick boundary when a [`GameInput::PlayerMove`] carried
    /// non-finite or out-of-range coordinates and no valid move superseded it in
    /// the same tick. Carries the player's last accepted position so the session
    /// layer can send a correction to the desynced client.
    PlayerPositionCorrected {
        /// Identity of the player to correct.
        player: PlayerId,
        /// Authoritative position the client must return to.
        position: Vec3,
    },
    /// A player was removed from the shard.
    PlayerDespawned {
        /// Identity of the despawned player.
        player: PlayerId,
    },
    /// The block at `position` changed to `state`.
    ///
    /// Emitted at the tick boundary after the simulation applies an accepted
    /// block break (`state` is [`BlockStateId::AIR`]) or place. The session
    /// layer broadcasts it to viewers in range as a clientbound `BlockUpdate`
    /// and, for a [`MutationCause::PlayerCreative`] edit, echoes `sequence` back
    /// to the acting player as an `AcknowledgeBlockChange`.
    BlockChanged {
        /// Absolute position of the changed block.
        position: BlockPos,
        /// The block's new state.
        state: BlockStateId,
        /// The originating block-action sequence to acknowledge to the actor.
        sequence: i32,
        /// What caused the mutation (carries the acting player for the ack).
        cause: MutationCause,
    },
    /// Completes the prediction sequence of a mining start/abort/early finish.
    BlockActionAcknowledged {
        /// Recipient of the mandatory acknowledgement.
        player: PlayerId,
        /// Original client prediction sequence.
        sequence: i32,
    },
    /// Changes the block-destruction overlay, with `-1` clearing it.
    MiningProgress {
        /// Actor whose actual session wire identity names the overlay.
        player: PlayerId,
        /// Absolute target cell.
        position: BlockPos,
        /// Crack stage `0..=9`, or `-1` to remove it.
        stage: i8,
    },
    /// An actual Survival break committed through the block-edit funnel.
    ///
    /// App owns the drop-spawn, tool-wear, experience and after-event consumers.
    /// The complete stacks include former chest contents without stripping
    /// components. These are move-owned outputs, not inventory grants or durable
    /// commit acknowledgements; native ownership never emits this legacy output.
    SurvivalBlockBroken {
        /// Actor who completed mining.
        player: PlayerId,
        /// Changed primary cell.
        position: BlockPos,
        /// Sequence of the completed start or finish action.
        sequence: i32,
        /// Exact block preimage used to derive drops.
        previous_state: BlockStateId,
        /// Registry held-item snapshot to guard the application's wear.
        held_item: Option<i32>,
        /// Immutable application witness for exact held-stack wear.
        tool_token: u64,
        /// Complete stacks to admit through the existing concrete entity owner.
        drops: Vec<ferrumc_items::ItemStack>,
        /// Sampled ordinary ore experience, before any plugin adjustment.
        experience: u32,
        /// Ordinary tool damage, zero for bare hands and zero-hardness blocks.
        tool_damage: u8,
    },
    /// A player's block edit was rejected; the actor's predicted change must heal
    /// to the authoritative state.
    ///
    /// Emitted at the tick boundary when an edit that has a client to heal is
    /// refused — the actor is present and the target chunk is resident, so the
    /// authoritative state is readable (e.g. the target is out of reach). The
    /// session layer sends only the actor a `BlockUpdate` carrying
    /// `authoritative_state` at `position` *followed by* an
    /// `AcknowledgeBlockChange` echoing `sequence`: the `BlockUpdate` sets the
    /// client's known server state and the ack ends its pending prediction so
    /// that state is what it displays. The ack is what actually reverts the ghost
    /// block on a real 1.21.8 client (a `BlockUpdate` alone is swallowed while a
    /// prediction is pending), so it is mandatory on reject just as on accept.
    /// Nothing is broadcast (viewers never saw the predicted change). Rejections
    /// with no client to heal (absent actor, unloaded chunk) emit nothing.
    BlockChangeRejected {
        /// The player whose predicted edit must be undone.
        player: PlayerId,
        /// Absolute position of the rejected edit.
        position: BlockPos,
        /// The originating block-action sequence to acknowledge to the actor, so
        /// its pending client-side prediction ends and reverts to the
        /// authoritative state.
        sequence: i32,
        /// The state the client optimistically predicted (air for a break, the
        /// placed block for a place); used by the app to classify the metric.
        requested_state: BlockStateId,
        /// The authoritative state to resync the actor to.
        authoritative_state: BlockStateId,
    },
    /// A sign block-entity's text was created or changed and must be (re)rendered
    /// for everyone who can see it.
    ///
    /// Emitted at the tick boundary after an accepted [`GameInput::UpdateSign`].
    /// The session layer encodes `sign` into the network NBT and broadcasts a
    /// `BlockEntityData` (droppable) to every viewer within view distance of
    /// `position`, so the sign renders its new text. The full sign snapshot is
    /// carried (not just the edited face) so the carrier is self-contained.
    SignUpdated {
        /// Absolute position of the sign.
        position: BlockPos,
        /// The sign's full post-edit state (both faces). Boxed to keep
        /// [`GameOutput`] small (a [`Sign`] holds eight text lines).
        sign: Box<Sign>,
    },
    /// Final tick-boundary result of one player placement input.
    ///
    /// Emitted exactly once per `BlockPlace` or `SetBlockExactTracked` when the shard opts
    /// into placement completions. It follows all edits/corrections for the
    /// action, including multi-cell writes and relocated merges. Session routing
    /// does not serialize this internal completion as a client packet.
    PlayerPlacementResolved {
        /// Actor, including an absent actor whose input was refused.
        player: PlayerId,
        /// Actual primary cell on success; original requested cell on refusal.
        position: BlockPos,
        /// Original action sequence.
        sequence: i32,
        /// Final primary state if applied; None for every refusal.
        state: Option<BlockStateId>,
    },
    /// A player placed a sign and should be shown its editing screen.
    ///
    /// Emitted at the tick boundary after an accepted [`GameInput::BlockPlace`]
    /// (or exact write) whose resulting block is a sign and whose block-entity was
    /// just created. The session layer sends only `player` an `OpenSignEditor`
    /// targeting `position`'s front face, opening the client's sign-edit screen;
    /// the client replies with an `UpdateSign` when the player confirms.
    OpenSignEditor {
        /// The player to open the editor for (the placer).
        player: PlayerId,
        /// Absolute position of the just-placed sign.
        position: BlockPos,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_compare_by_value() {
        let p = PlayerId::offline("notch");
        let a = GameInput::PlayerJoin {
            player: p,
            position: Vec3::new(1.0, 2.0, 3.0),
        };
        let b = a.clone();
        assert_eq!(a, b);
        assert_ne!(
            a,
            GameInput::PlayerMove {
                player: p,
                position: Some(Vec3::new(1.0, 2.0, 3.0)),
                yaw: None,
                pitch: None,
            }
        );
    }

    #[test]
    fn outputs_compare_by_value() {
        let p = PlayerId::offline("jeb_");
        let spawned = GameOutput::PlayerSpawned {
            player: p,
            position: Vec3::ZERO,
        };
        assert_eq!(spawned.clone(), spawned);
        assert_ne!(spawned, GameOutput::PlayerDespawned { player: p });
    }
}
