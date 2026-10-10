//! Concrete native player inventories and receipt-confirmed owner operations.
//!
//! Data snapshots grant no live authority. The shard owns the actual slots and
//! cursor; the application must use this explicitly selected consumer before
//! replacing its legacy connection inventory.

mod checkpoint;
pub(crate) mod codec;
pub(crate) mod evidence;
pub(crate) mod pending;
pub(crate) mod plan;
#[cfg(test)]
mod tests;

use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::{Arc, Weak};

use ferrumc_core::{GameMode, PlayerId};
use ferrumc_items::{ItemPersistenceError, ItemStack, StackTransferError};
use ferrumc_math::Vec3;
use ferrumc_storage::{OwnershipError, PlayerRecord, SchemaVersion};

use crate::{EntityStoreError, ItemEntityError};

pub use checkpoint::OwnershipCheckpointResolution;
pub use evidence::{
    OwnedEntityRead, OwnedPairRead, OwnedPlayerRead, OwnershipStoreBinding,
    PlayerOwnershipCompletion, PlayerOwnershipStorageWork, PlayerOwnershipWorkRefusal,
};
pub use pending::{
    DropSelection, NormalSlotButton, OwnedPlayerInput, OwnedPlayerInputRefusal, OwnershipIntent,
    OwnershipPendingCheckpoint, OwnershipPendingStatus, PlayerOwnershipTickOutcome,
};
pub(crate) use pending::{OwnedPlayerInventory, PendingPlayerMutation, PlayerOwnershipState};

/// Original window-0 slots, including recipe output, crafting, armor and offhand.
pub const NATIVE_PLAYER_SLOT_COUNT: usize = 46;
/// Exact fixed FCPS version-3 header length.
pub const NATIVE_PLAYER_HEADER_BYTES: usize = 41;
/// Payload schema used by the existing FCPS version-3 application codec.
pub const NATIVE_PLAYER_SCHEMA: SchemaVersion = SchemaVersion::new(3);
/// Maximum explicit owner-local player registrations.
pub const MAX_OWNED_PLAYERS: usize = 128;
/// Maximum aggregate checked encoded player bytes in one shard.
pub const MAX_OWNED_PLAYER_BYTES: usize = 32 * 1024 * 1024;
/// Maximum retained ownership operations in one shard.
pub const MAX_PLAYER_OWNERSHIP_PENDING: usize = 32;
/// Maximum retained pending representation charge, including clone allowance.
pub const MAX_PLAYER_OWNERSHIP_PENDING_BYTES: usize = 8 * 1024 * 1024;
/// Maximum simultaneous queued concrete inputs and completions.
pub const MAX_PLAYER_OWNERSHIP_INPUTS: usize = 32;
/// Maximum checked encoded charge of the complete ownership inbox.
pub const MAX_PLAYER_OWNERSHIP_INPUT_BYTES: usize = 8 * 1024 * 1024;

/// Classified native-state, owner, planning or publication refusal.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlayerOwnershipError {
    /// A positive resource agreement exceeded its hard ceiling.
    #[error("invalid player ownership limit: {0}")]
    InvalidLimit(&'static str),
    /// Native payload schema is not FCPS3; migration belongs to the driver.
    #[error("unsupported native player schema {0}")]
    UnsupportedSchema(SchemaVersion),
    /// A concrete envelope field is malformed, without normalization.
    #[error("malformed FCPS3 field: {0}")]
    Malformed(&'static str),
    /// A nested FCIS stack is malformed or outside its supported model.
    #[error(transparent)]
    Item(#[from] ItemPersistenceError),
    /// One pure lossless split or transfer refused its source/destination.
    #[error(transparent)]
    Transfer(#[from] StackTransferError),
    /// The real storage producer refused or reported an uncertain commit.
    #[error(transparent)]
    Storage(#[from] OwnershipError),
    /// Native item admission or persistence preflight failed.
    #[error(transparent)]
    Entity(#[from] ItemEntityError),
    /// Existing owner-local slot admission or lifecycle gate refused.
    #[error(transparent)]
    EntityStore(#[from] EntityStoreError),
    /// Checked chunk lifecycle refused without dropping player/entity authority.
    #[error(transparent)]
    Chunk(#[from] crate::SimError),
    /// A count/encoded representation cannot fit its explicit allowance.
    #[error("player ownership capacity exceeded: {0}")]
    Capacity(&'static str),
    /// Allocation refused before an ownership move.
    #[error("bounded player ownership allocation failed")]
    AllocationFailed,
    /// This shard has no opted-in concrete player authority.
    #[error("player ownership is not initialized")]
    NotInitialized,
    /// Opt-in configuration cannot replace an existing live owner or snapshot.
    #[error("player ownership initialization would replace an owner")]
    AlreadyInitialized,
    /// A copied shard contains data only, with no activation permission.
    #[error("ownership snapshot has no live write authority")]
    SnapshotOnly,
    /// Current generation/map/key scope differs from its explicit binding.
    #[error("player ownership scope or generation identity differs")]
    ScopeMismatch,
    /// A helper was given another backend allocation, even if contents match.
    #[error("player ownership backend allocation differs")]
    ForeignStore,
    /// Handle names another shard incarnation or player admission.
    #[error("player ownership handle is foreign or stale")]
    StaleHandle,
    /// The UUID is already locally admitted, including detached/pending players.
    #[error("player is already owned by this shard")]
    AlreadyOwned,
    /// The complete player or entity mutation boundary is already reserved.
    #[error("player or entity ownership is reserved")]
    Reserved,
    /// The caller's last publication no longer names the current player state.
    #[error("player local generation is stale")]
    StaleGeneration,
    /// A read no longer agrees with the owner's confirmed durable baseline.
    #[error("storage snapshot differs from confirmed owner state")]
    StaleSnapshot,
    /// ID, digest, owners or before/after evidence do not match the pending work.
    #[error("ownership completion does not match its reservation")]
    CompletionMismatch,
    /// A committed receipt is historical and cannot overwrite a newer postimage.
    #[error("ownership receipt postimage was superseded")]
    Superseded,
    /// A counter cannot safely advance without wrapping into an old authority.
    #[error("player local authority generation exhausted")]
    GenerationExhausted,
    /// This exact item policy cannot yet resolve effective default components.
    #[error("effective item component policy is unresolved: {0}")]
    UnresolvedComponentPolicy(i32),
    /// The actor or native item is ineligible for the selected concrete action.
    #[error("player ownership action is ineligible: {0}")]
    Ineligible(&'static str),
    /// New intents cannot enter a draining or stopped ownership consumer.
    #[error("player ownership is draining or stopped")]
    Draining,
    /// Pending work or an unsaved native player prevents a successful stop.
    #[error("player ownership has pending or unsaved work")]
    NotQuiescent,
    /// Absence of a receipt does not establish that an old worker has stopped.
    #[error("receipt absent without proof of the old worker lifetime")]
    UnprovenWorkerLifetime,
}

/// Explicit positive owner and queue allowances, bounded before allocation.
#[derive(Debug, Clone, Copy)]
pub struct PlayerOwnershipLimits {
    pub(crate) players: usize,
    pub(crate) player_bytes: usize,
    pub(crate) pending: usize,
    pub(crate) pending_bytes: usize,
    pub(crate) inputs: usize,
    pub(crate) input_bytes: usize,
}

impl PlayerOwnershipLimits {
    /// Declares every count and encoded-byte allowance without allocation.
    pub fn new(
        players: NonZeroUsize,
        player_bytes: NonZeroUsize,
        pending: NonZeroUsize,
        pending_bytes: NonZeroUsize,
        inputs: NonZeroUsize,
        input_bytes: NonZeroUsize,
    ) -> Result<Self, PlayerOwnershipError> {
        for (value, max, field) in [
            (players.get(), MAX_OWNED_PLAYERS, "players"),
            (player_bytes.get(), MAX_OWNED_PLAYER_BYTES, "player bytes"),
            (pending.get(), MAX_PLAYER_OWNERSHIP_PENDING, "pending"),
            (
                pending_bytes.get(),
                MAX_PLAYER_OWNERSHIP_PENDING_BYTES,
                "pending bytes",
            ),
            (inputs.get(), MAX_PLAYER_OWNERSHIP_INPUTS, "inputs"),
            (
                input_bytes.get(),
                MAX_PLAYER_OWNERSHIP_INPUT_BYTES,
                "input bytes",
            ),
        ] {
            if value > max {
                return Err(PlayerOwnershipError::InvalidLimit(field));
            }
        }
        Ok(Self {
            players: players.get(),
            player_bytes: player_bytes.get(),
            pending: pending.get(),
            pending_bytes: pending_bytes.get(),
            inputs: inputs.get(),
            input_bytes: input_bytes.get(),
        })
    }
}

/// Positive owner-local publication generation; independent of durable revisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerLocalGeneration(pub(crate) NonZeroU64);

impl PlayerLocalGeneration {
    /// Returns the nonwrapping local generation number.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
    pub(crate) fn next(self) -> Result<Self, PlayerOwnershipError> {
        self.get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(Self)
            .ok_or(PlayerOwnershipError::GenerationExhausted)
    }
}

/// Opaque local player admission, with no window-ID or storage-token authority.
#[derive(Clone)]
pub struct OwnedPlayerHandle {
    pub(crate) owner: Weak<()>,
    pub(crate) player: PlayerId,
    pub(crate) admission: PlayerLocalGeneration,
}

impl std::fmt::Debug for OwnedPlayerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedPlayerHandle")
            .field("player", &self.player)
            .field("admission", &self.admission)
            .finish_non_exhaustive()
    }
}
impl PartialEq for OwnedPlayerHandle {
    fn eq(&self, other: &Self) -> bool {
        self.player == other.player
            && self.admission == other.admission
            && Weak::ptr_eq(&self.owner, &other.owner)
    }
}
impl Eq for OwnedPlayerHandle {}
impl OwnedPlayerHandle {
    /// Returns the globally identified player, without granting another admission.
    pub const fn player(&self) -> PlayerId {
        self.player
    }
}

/// Move-owned complete native FCPS3 data; construction grants no live authority.
#[derive(Debug)]
pub struct NativePlayerSnapshot {
    pub(crate) position: Vec3,
    pub(crate) look: [f32; 2],
    pub(crate) game_mode: GameMode,
    pub(crate) selected: u8,
    pub(crate) slots: Box<[ItemStack; NATIVE_PLAYER_SLOT_COUNT]>,
    pub(crate) carried: ItemStack,
}

impl NativePlayerSnapshot {
    /// Retains supplied data as a pure value; encode/admission validates all fields.
    pub fn new(
        position: Vec3,
        look: [f32; 2],
        game_mode: GameMode,
        selected: u8,
        slots: Box<[ItemStack; NATIVE_PLAYER_SLOT_COUNT]>,
        carried: ItemStack,
    ) -> Self {
        Self {
            position,
            look,
            game_mode,
            selected,
            slots,
            carried,
        }
    }
    /// Creates empty Survival data only for an explicitly confirmed fresh bootstrap.
    pub fn empty_survival(position: Vec3, look: [f32; 2]) -> Self {
        Self::new(
            position,
            look,
            GameMode::Survival,
            0,
            Box::new(std::array::from_fn(|_| ItemStack::empty())),
            ItemStack::empty(),
        )
    }
    /// Returns the exact native position bits.
    pub const fn position(&self) -> Vec3 {
        self.position
    }
    /// Returns body yaw and pitch without normalization.
    pub const fn look(&self) -> [f32; 2] {
        self.look
    }
    /// Returns the typed mode stored beside the opaque FCPS payload.
    pub const fn game_mode(&self) -> GameMode {
        self.game_mode
    }
    /// Returns the selected hotbar index; admission requires `0..=8`.
    pub const fn selected(&self) -> u8 {
        self.selected
    }
    /// Borrows every original window-0 slot, including the read-only recipe output.
    pub fn slots(&self) -> &[ItemStack; NATIVE_PLAYER_SLOT_COUNT] {
        &self.slots
    }
    /// Borrows the one carried stack, including while no GUI is open.
    pub const fn carried(&self) -> &ItemStack {
        &self.carried
    }
    /// Encodes the existing exact bounded schema, refusing rather than normalizing.
    pub fn try_to_record(&self) -> Result<PlayerRecord, PlayerOwnershipError> {
        codec::encode(self)
    }
    /// Reads FCPS3 only; explicit legacy JSON migration belongs to the driver.
    pub fn try_from_record(record: &PlayerRecord) -> Result<Self, PlayerOwnershipError> {
        codec::decode(record)
    }
}

pub(crate) fn same_owner(owner: &Weak<()>, actual: &Arc<()>) -> bool {
    Weak::ptr_eq(owner, &Arc::downgrade(actual)) && owner.strong_count() != 0
}
