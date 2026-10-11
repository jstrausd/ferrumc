//! Bounded concrete player inputs, pending postimages and publication intents.

use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroU64;
use std::sync::Arc;

use ferrumc_core::{EntityId, EntityUuid, GameMode, PlayerId};
use ferrumc_items::{ItemId, ItemStack};
use ferrumc_math::Vec3;
use ferrumc_storage::{
    OwnedEntityExpectation, OwnedPlayerExpectation, OwnershipReceiptQuery, OwnershipTransferId,
    PersistentEntityRecord, PlayerRecord,
};

use super::evidence::{CompletionTicket, WorkRequest};
use super::{
    NativePlayerSnapshot, OwnedEntityRead, OwnedPairRead, OwnedPlayerHandle, OwnedPlayerRead,
    OwnershipStoreBinding, PlayerLocalGeneration, PlayerOwnershipCompletion, PlayerOwnershipError,
    PlayerOwnershipLimits, PlayerOwnershipStorageWork, NATIVE_PLAYER_SLOT_COUNT,
};
use crate::entity::ownership::PreparedOwnedEntity;
use crate::{EntityHandle, GameOutput, ItemEntitySpawn};

/// Original selected-hand drop quantity, separate from arbitrary GUI drops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropSelection {
    /// At most one item from the currently selected hotbar stack.
    One,
    /// The complete selected stack.
    All,
}
/// The supported ordinary window-0 slot click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalSlotButton {
    /// Pick up, deposit, merge or swap the complete held quantity.
    Left,
    /// Pick up the larger half or deposit at most one item.
    Right,
}

#[derive(Debug)]
pub(crate) enum InputKind {
    EntitySave {
        item: EntityHandle,
        generation: u64,
        id: OwnershipTransferId,
    },
    RefillItemSaveIds {
        owner: crate::OwnedItemCounterHandle,
        ids: Vec<OwnershipTransferId>,
    },
    RestorePlayer(OwnedPlayerRead),
    Bootstrap {
        read: OwnedPlayerRead,
        position: Vec3,
        look: [f32; 2],
        id: OwnershipTransferId,
    },
    RestoreItem {
        read: OwnedEntityRead,
        wire_id: EntityId,
    },
    SpawnItem {
        read: OwnedEntityRead,
        proposal: ItemEntitySpawn,
        id: OwnershipTransferId,
    },
    Drop {
        player: OwnedPlayerHandle,
        read: OwnedPairRead,
        selection: DropSelection,
        wire_id: EntityId,
        id: OwnershipTransferId,
        seed: i64,
        cancelled: bool,
    },
    Pickup {
        player: OwnedPlayerHandle,
        item: EntityHandle,
        read: OwnedPairRead,
        id: OwnershipTransferId,
        animate: bool,
        cancelled: bool,
    },
    Autosave {
        player: OwnedPlayerHandle,
        id: OwnershipTransferId,
    },
    Click {
        player: OwnedPlayerHandle,
        generation: PlayerLocalGeneration,
        slot: usize,
        button: NormalSlotButton,
        id: OwnershipTransferId,
    },
    Move {
        player: OwnedPlayerHandle,
        position: Option<Vec3>,
        look: [Option<f32>; 2],
    },
    Mode {
        player: OwnedPlayerHandle,
        mode: GameMode,
    },
    Select {
        player: OwnedPlayerHandle,
        selected: u8,
    },
    Disconnect(OwnedPlayerHandle),
    Rejoin(OwnedPlayerRead),
    Completion(PlayerOwnershipCompletion),
    Retry(OwnershipReceiptQuery),
    Reconcile(OwnershipReceiptQuery),
    Drain,
    Stop,
}

/// Move-owned concrete input; opaque constructors never confer live authority.
#[derive(Debug)]
pub struct OwnedPlayerInput(pub(crate) InputKind);
impl OwnedPlayerInput {
    /// Captures a concrete item save at its current positive local generation.
    /// The native confirmed beforeimage remains owned by its existing slot.
    pub fn save_item(item: EntityHandle, generation: u64, id: OwnershipTransferId) -> Self {
        Self(InputKind::EntitySave {
            item,
            generation,
            id,
        })
    }

    /// Supplies whole move-owned save IDs to the concrete counter incarnation.
    /// Refused admission returns the entire refill, with no ID consumed.
    pub fn refill_item_save_ids(
        owner: crate::OwnedItemCounterHandle,
        ids: Vec<OwnershipTransferId>,
    ) -> Self {
        Self(InputKind::RefillItemSaveIds { owner, ids })
    }

    /// Requests admission of a real stored FCPS3 player at the next owner boundary.
    pub fn restore(read: OwnedPlayerRead) -> Self {
        Self(InputKind::RestorePlayer(read))
    }
    /// Requests empty Survival bootstrap from a confirmed never-seen storage miss.
    pub fn bootstrap(
        read: OwnedPlayerRead,
        position: Vec3,
        look: [f32; 2],
        id: OwnershipTransferId,
    ) -> Self {
        Self(InputKind::Bootstrap {
            read,
            position,
            look,
            id,
        })
    }
    /// Restores one real FCIE item with a newly supplied wire ID.
    pub fn restore_item(read: OwnedEntityRead, wire_id: EntityId) -> Self {
        Self(InputKind::RestoreItem { read, wire_id })
    }
    /// Requests a receipt-backed new item spawn, retaining every proposal on refusal.
    pub fn spawn_item(
        read: OwnedEntityRead,
        proposal: ItemEntitySpawn,
        id: OwnershipTransferId,
    ) -> Self {
        Self(InputKind::SpawnItem { read, proposal, id })
    }
    /// Plans selected-hand drop once; the explicit seed is captured rather than rerolled.
    ///
    /// Native cancellation leaves the complete selected stack intact. The pose
    /// policy in this first consumer is the explicitly documented standing pose.
    pub fn drop_selected(
        player: OwnedPlayerHandle,
        read: OwnedPairRead,
        selection: DropSelection,
        wire_id: EntityId,
        id: OwnershipTransferId,
        seed: i64,
        cancelled: bool,
    ) -> Self {
        Self(InputKind::Drop {
            player,
            read,
            selection,
            wire_id,
            id,
            seed,
            cancelled,
        })
    }
    /// Plans actual eligible item pickup; animation and native cancellation are explicit.
    pub fn pickup(
        player: OwnedPlayerHandle,
        item: EntityHandle,
        read: OwnedPairRead,
        id: OwnershipTransferId,
        animate: bool,
        cancelled: bool,
    ) -> Self {
        Self(InputKind::Pickup {
            player,
            item,
            read,
            id,
            animate,
            cancelled,
        })
    }
    /// Captures the current complete actual player for a receipt-backed single CAS.
    pub fn autosave(player: OwnedPlayerHandle, id: OwnershipTransferId) -> Self {
        Self(InputKind::Autosave { player, id })
    }
    /// Requests a conserving ordinary slot/cursor click against the last publication.
    pub fn normal_slot_click(
        player: OwnedPlayerHandle,
        generation: PlayerLocalGeneration,
        slot: usize,
        button: NormalSlotButton,
        id: OwnershipTransferId,
    ) -> Self {
        Self(InputKind::Click {
            player,
            generation,
            slot,
            button,
            id,
        })
    }
    /// Requests checked movement/look; pending transfers retain one coalesced last value.
    pub fn move_player(
        player: OwnedPlayerHandle,
        position: Option<Vec3>,
        look: [Option<f32>; 2],
    ) -> Self {
        Self(InputKind::Move {
            player,
            position,
            look,
        })
    }
    /// Requests an owner-local mode change, deferred while the full postimage is reserved.
    pub fn set_game_mode(player: OwnedPlayerHandle, mode: GameMode) -> Self {
        Self(InputKind::Mode { player, mode })
    }
    /// Requests a strict hotbar selection, deferred while pending.
    pub fn select_hotbar(player: OwnedPlayerHandle, selected: u8) -> Self {
        Self(InputKind::Select { player, selected })
    }
    /// Detaches visibility without dropping the inventory or pending authority.
    pub fn disconnect(player: OwnedPlayerHandle) -> Self {
        Self(InputKind::Disconnect(player))
    }
    /// Requests a new local admission for a detached, fully clarified player.
    pub fn rejoin(read: OwnedPlayerRead) -> Self {
        Self(InputKind::Rejoin(read))
    }
    /// Submits sealed genuine backend evidence for the next completion prefix.
    pub fn complete(completion: PlayerOwnershipCompletion) -> Self {
        Self(InputKind::Completion(completion))
    }
    /// Requests the identical retained request only where retry is explicitly eligible.
    pub fn retry(query: OwnershipReceiptQuery) -> Self {
        Self(InputKind::Retry(query))
    }
    /// Requests readonly receipt/current clarification, never treating None as an abort.
    pub fn reconcile(query: OwnershipReceiptQuery) -> Self {
        Self(InputKind::Reconcile(query))
    }
    /// Stops admission of new gameplay intents and retains all pending work.
    pub fn drain() -> Self {
        Self(InputKind::Drain)
    }
    /// Requests terminal stop only after all pending/dirty player work is clarified.
    pub fn stop() -> Self {
        Self(InputKind::Stop)
    }
    pub(crate) fn charge(&self) -> Result<usize, PlayerOwnershipError> {
        let bytes = match &self.0 {
            InputKind::RefillItemSaveIds { ids, .. } => {
                if ids.len() > 32 || ids.capacity() > 32 {
                    return Err(PlayerOwnershipError::Capacity("save ID input"));
                }
                ids.len()
                    .checked_mul(16)
                    .ok_or(PlayerOwnershipError::Capacity("save ID input"))?
            }
            InputKind::RestorePlayer(r)
            | InputKind::Rejoin(r)
            | InputKind::Bootstrap { read: r, .. } => {
                r.snapshot.record().map_or(0, |r| r.data().len())
            }
            InputKind::RestoreItem { read, .. } => read
                .snapshot
                .record()
                .map_or(0, |r| r.record().data().len()),
            InputKind::SpawnItem { read, proposal, .. } => {
                read.snapshot
                    .record()
                    .map_or(0, |r| r.record().data().len())
                    + ferrumc_items::persisted_stack_size(proposal.stack())?
                    + crate::NATIVE_ITEM_ENTITY_HEADER_BYTES
            }
            InputKind::Drop { read, .. } | InputKind::Pickup { read, .. } => {
                read.snapshot
                    .player()
                    .record()
                    .map_or(0, |r| r.data().len())
                    + read
                        .snapshot
                        .entity()
                        .record()
                        .map_or(0, |r| r.record().data().len())
            }
            InputKind::Completion(c) => return Ok(c.charged_bytes()),
            _ => 0,
        };
        bytes
            .checked_add(512)
            .ok_or(PlayerOwnershipError::Capacity("input bytes"))
    }
}

/// Refusal that retains the entire original input, including owned spawn items.
#[derive(Debug)]
pub struct OwnedPlayerInputRefusal {
    pub(crate) reason: PlayerOwnershipError,
    pub(crate) input: Box<OwnedPlayerInput>,
}
impl OwnedPlayerInputRefusal {
    /// Borrows the precise refusal without disposing of caller-owned data.
    pub const fn reason(&self) -> &PlayerOwnershipError {
        &self.reason
    }
    /// Recovers the exact original input and its typed reason.
    pub fn into_parts(self) -> (PlayerOwnershipError, OwnedPlayerInput) {
        (self.reason, *self.input)
    }
}

/// Concrete retained-operation phase, never inferred from wall time or receipt absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipPendingStatus {
    /// An exact request can be dispatched, including after a real admission Busy.
    Prepared,
    /// Work has left the owner and still needs evidence.
    InFlight,
    /// A real started operation may have committed, or its caller was lost.
    Uncertain,
    /// A receipt confirms commit but a current/native/lifecycle read must still succeed.
    NeedsRefresh,
    /// Historical or mismatched evidence requires explicit resync/recovery.
    Quarantined,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PendingKind {
    Bootstrap,
    Autosave,
    Click,
    Drop,
    Pickup,
    SpawnItem,
    EntityAutosave,
    EntityDespawn,
}

#[derive(Debug, Clone)]
pub(crate) struct OwnedPlayerInventory {
    pub(crate) slots: Box<[ItemStack; NATIVE_PLAYER_SLOT_COUNT]>,
    pub(crate) carried: ItemStack,
    pub(crate) selected: u8,
    pub(crate) expected: OwnedPlayerExpectation,
    pub(crate) confirmed: PlayerRecord,
    pub(crate) admission: PlayerLocalGeneration,
    pub(crate) mutation: PlayerLocalGeneration,
    pub(crate) connected: bool,
    pub(crate) dirty: bool,
    pub(crate) pending: Option<OwnershipTransferId>,
    pub(crate) deferred_position: Option<Vec3>,
    pub(crate) deferred_look: [Option<f32>; 2],
    pub(crate) deferred_mode: Option<GameMode>,
    pub(crate) deferred_selected: Option<u8>,
}

#[derive(Debug)]
pub(crate) struct PendingPlayerMutation {
    pub(crate) lifecycle_cell: Option<crate::collector_lifecycle::LifecycleCellKey>,
    pub(crate) publication: Option<crate::item_publication::PreparedItemPublication>,
    pub(crate) buffer: std::sync::Weak<()>,
    pub(crate) kind: PendingKind,
    pub(crate) ticket: CompletionTicket,
    pub(crate) request: WorkRequest,
    pub(crate) player: Option<PlayerId>,
    pub(crate) before_player: Option<PlayerRecord>,
    pub(crate) before_entity: Option<PersistentEntityRecord>,
    pub(crate) player_expected: Option<OwnedPlayerExpectation>,
    pub(crate) entity_expected: Option<OwnedEntityExpectation>,
    pub(crate) after_player: Option<NativePlayerSnapshot>,
    pub(crate) after_player_record: Option<PlayerRecord>,
    pub(crate) after_entity: Option<PreparedOwnedEntity>,
    pub(crate) after_entity_record: Option<PersistentEntityRecord>,
    pub(crate) local_before: Option<PlayerLocalGeneration>,
    pub(crate) status: OwnershipPendingStatus,
    pub(crate) charge: usize,
    pub(crate) moved: u8,
    pub(crate) original_count: u8,
    pub(crate) stat_item: Option<ItemId>,
    pub(crate) animate: bool,
    pub(crate) item_generation: Option<u64>,
    pub(crate) counters: Option<crate::item_counters::DeferredOwnedCounters>,
}

/// Checked version-1 retained data, without live owner, CAS or completion authority.
///
/// Durable checkpoint wiring is the driver's responsibility. In particular this
/// object cannot prove that a worker from a previous driver/process has ended.
#[derive(Debug)]
pub struct OwnershipPendingCheckpoint {
    pub(crate) query: OwnershipReceiptQuery,
    pub(crate) binding: crate::GenerationBinding,
    pub(crate) player_key: Option<ferrumc_storage::ScopedPlayerKey>,
    pub(crate) entity_key: Option<ferrumc_storage::PersistentEntityKey>,
    pub(crate) player_stamp: Option<ferrumc_storage::OwnershipStamp>,
    pub(crate) entity_stamp: Option<ferrumc_storage::OwnershipStamp>,
    pub(crate) kind: PendingKind,
    pub(crate) before_player: Option<PlayerRecord>,
    pub(crate) before_entity: Option<PersistentEntityRecord>,
    pub(crate) after_player: Option<PlayerRecord>,
    pub(crate) after_entity: Option<PersistentEntityRecord>,
    pub(crate) status: OwnershipPendingStatus,
}
impl OwnershipPendingCheckpoint {
    /// Returns the concrete checkpoint version, independent of storage formats.
    pub const fn version(&self) -> u16 {
        1
    }
    /// Returns the exact original operation identity, without issuing a new request.
    pub const fn query(&self) -> OwnershipReceiptQuery {
        self.query
    }
    /// Returns why the reservation remains retained.
    pub const fn status(&self) -> OwnershipPendingStatus {
        self.status
    }
}

/// Actual owner publication, resync, stat or lifecycle intent after checked work.
#[derive(Debug)]
pub enum OwnershipIntent {
    /// Automatic saving needs move-owned driver IDs; at most one per boundary.
    SaveIdsNeeded,
    /// Concrete save selection could not be admitted; dirty authority remains.
    ItemSaveBlocked {
        /// Actual locally selected item.
        item: EntityHandle,
        /// Specific resource or authority refusal.
        reason: PlayerOwnershipError,
    },

    /// Definitive single-spawn refusal returns the entire caller-owned proposal.
    ItemSpawnRefused {
        /// Original durable operation identity.
        query: OwnershipReceiptQuery,
        /// Complete unchanged caller proposal, with no visible/durable spawn.
        proposal: ItemEntitySpawn,
    },
    /// One actual complete player admission became visible.
    PlayerAdmitted {
        /// Actual actor or current local admission.
        player: OwnedPlayerHandle,
        /// Exact nonwrapping local publication generation.
        generation: PlayerLocalGeneration,
    },
    /// Inventory/mode/selection changed at this exact local generation.
    PlayerChanged {
        /// Actual actor or current local admission.
        player: OwnedPlayerHandle,
        /// Exact nonwrapping local publication generation.
        generation: PlayerLocalGeneration,
    },
    /// Actor must refresh the actual readonly owner image; no speculative items.
    Resync {
        /// Actual actor or current local admission.
        player: OwnedPlayerHandle,
        /// Exact nonwrapping local publication generation.
        generation: PlayerLocalGeneration,
    },
    /// An item became visible only after a confirmed single or pair commit.
    ItemSpawned {
        /// Original durable operation ID and digest.
        query: OwnershipReceiptQuery,
        /// Actual item handle or reference.
        item: EntityHandle,
    },
    /// A real stored item was restored with its explicit newly allocated wire identity.
    ItemRestored {
        /// Actual item handle or reference.
        item: EntityHandle,
    },
    /// A partial pickup published an updated item in the same owner slot.
    ItemChanged {
        /// Original durable operation ID and digest.
        query: OwnershipReceiptQuery,
        /// Actual item handle or reference.
        item: EntityHandle,
    },
    /// Full pickup published removal after the confirmed durable tombstone.
    ItemRemoved {
        /// Original durable operation ID and digest.
        query: OwnershipReceiptQuery,
        /// Removed durable entity identity.
        uuid: EntityUuid,
        /// Removed runtime entity reference.
        wire_id: EntityId,
    },
    /// The original Paper count for take animation, distinct from actual moved count.
    Take {
        /// Original durable operation ID and digest.
        query: OwnershipReceiptQuery,
        /// Actual actor or current local admission.
        player: PlayerId,
        /// Actual item handle or reference.
        item: EntityId,
        /// Original reference quantity for this intent.
        count: u8,
        /// Actual conserving inserted quantity.
        moved: u8,
    },
    /// Original reference stat intent, emitted only after confirmed publication.
    Stat {
        /// Original durable operation ID and digest.
        query: OwnershipReceiptQuery,
        /// Actual actor or current local admission.
        player: PlayerId,
        /// Actual item handle or reference.
        item: ItemId,
        /// Original reference quantity for this intent.
        count: u8,
        /// Whether this stat records a selected-hand drop.
        dropped: bool,
    },
    /// Disconnected player data and pending authority remain owned.
    PlayerDetached {
        /// Actual actor or current local admission.
        player: PlayerId,
    },
    /// New gameplay admission stopped; clarification/autosave remains available.
    Draining,
    /// Terminal stop succeeded after the actual consumer became quiescent.
    Stopped,
}

/// One bounded concrete ownership boundary plus the ordinary game outputs.
#[derive(Debug, Default)]
pub struct PlayerOwnershipTickOutcome {
    pub(crate) work: Vec<PlayerOwnershipStorageWork>,
    pub(crate) intents: Vec<OwnershipIntent>,
    pub(crate) refusals: Vec<OwnedPlayerInputRefusal>,
    pub(crate) game: Vec<GameOutput>,
    pub(crate) counter_steps: usize,
    pub(crate) save_scanned: usize,
}
impl PlayerOwnershipTickOutcome {
    /// Number of eligible real resident counter claims in this successful boundary.
    pub const fn counter_steps(&self) -> usize {
        self.counter_steps
    }
    /// Slots examined for save selection; this does not limit counter claims.
    pub const fn item_save_slots_scanned(&self) -> usize {
        self.save_scanned
    }

    /// Moves publication/refund intents to their concrete consumers without cloning.
    pub fn take_intents(&mut self) -> Vec<OwnershipIntent> {
        std::mem::take(&mut self.intents)
    }
    /// Moves exact requests to the external bounded storage driver.
    pub fn take_work(&mut self) -> Vec<PlayerOwnershipStorageWork> {
        std::mem::take(&mut self.work)
    }
    /// Borrows all confirmed publication and explicit resync/lifecycle intents.
    pub fn intents(&self) -> &[OwnershipIntent] {
        &self.intents
    }
    /// Moves intact refused caller inputs back to their owner.
    pub fn take_refusals(&mut self) -> Vec<OwnedPlayerInputRefusal> {
        std::mem::take(&mut self.refusals)
    }
    /// Borrows normal outputs from the same boundary's legacy game FIFO.
    pub fn game_outputs(&self) -> &[GameOutput] {
        &self.game
    }
}

#[derive(Debug)]
pub(crate) struct PlayerOwnershipState {
    pub(crate) lifecycle: Option<crate::collector_lifecycle::CollectorLifecycleState>,
    pub(crate) publications: Option<crate::item_publication::ItemPublicationState>,
    pub(crate) binding: OwnershipStoreBinding,
    pub(crate) limits: PlayerOwnershipLimits,
    pub(crate) owner: Arc<()>,
    pub(crate) snapshot_only: bool,
    pub(crate) generation: PlayerLocalGeneration,
    pub(crate) input: VecDeque<(OwnedPlayerInput, usize)>,
    pub(crate) input_bytes: usize,
    pub(crate) pending: BTreeMap<OwnershipTransferId, PendingPlayerMutation>,
    pub(crate) pending_bytes: usize,
    pub(crate) player_bytes: usize,
    pub(crate) draining: bool,
    pub(crate) stopped: bool,
    pub(crate) boundary_remaining: usize,
    pub(crate) item_counters: Option<crate::item_counters::OwnedItemCounterState>,
}
impl PlayerOwnershipState {
    pub(crate) fn new(binding: OwnershipStoreBinding, limits: PlayerOwnershipLimits) -> Self {
        Self {
            binding,
            publications: None,
            lifecycle: None,
            limits,
            owner: Arc::new(()),
            snapshot_only: false,
            generation: PlayerLocalGeneration(NonZeroU64::MIN),
            input: VecDeque::new(),
            input_bytes: 0,
            pending: BTreeMap::new(),
            pending_bytes: 0,
            player_bytes: 0,
            draining: false,
            stopped: false,
            boundary_remaining: 0,
            item_counters: None,
        }
    }
    pub(crate) fn next_generation(
        &mut self,
    ) -> Result<PlayerLocalGeneration, PlayerOwnershipError> {
        self.generation = self.generation.next()?;
        Ok(self.generation)
    }
}

impl Clone for PlayerOwnershipState {
    fn clone(&self) -> Self {
        // A snapshot carries no runnable input, worker ticket or reservation.
        // Copied inventory pending IDs remain descriptive data only.
        Self {
            binding: self.binding.clone(),
            publications: None,
            lifecycle: None,
            limits: self.limits,
            owner: Arc::new(()),
            snapshot_only: true,
            generation: self.generation,
            input: VecDeque::new(),
            input_bytes: 0,
            pending: BTreeMap::new(),
            pending_bytes: 0,
            player_bytes: self.player_bytes,
            draining: true,
            stopped: false,
            boundary_remaining: 0,
            item_counters: self
                .item_counters
                .as_ref()
                .map(crate::item_counters::OwnedItemCounterState::snapshot),
        }
    }
}
