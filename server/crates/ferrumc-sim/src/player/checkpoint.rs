//! Checked retained data without a resurrected worker or write capability.

use ferrumc_core::{DimensionId, EntityUuid, GameMode, PlayerId, WorldId};
use ferrumc_math::ChunkPos;
use ferrumc_storage::{
    EntityRecord, GenerationKey, OwnedEntityExpectation, OwnedPlayerExpectation,
    OwnershipReceiptQuery, OwnershipStamp, OwnershipTransferId, PersistentEntityKey,
    PersistentEntityRecord, PlayerEntityOwnershipStore, PlayerRecord, SchemaVersion,
    ScopedPlayerKey,
};
use ferrumc_world::{GenerationIdentity, GENERATION_IDENTITY_LEN};
use std::sync::Arc;

use super::evidence::check_receipt_epochs;
use super::pending::{PendingKind, PendingPlayerMutation};
use super::{
    NativePlayerSnapshot, OwnedEntityRead, OwnedPlayerRead, OwnershipPendingCheckpoint,
    OwnershipPendingStatus, OwnershipStoreBinding, PlayerOwnershipError,
};

impl PendingPlayerMutation {
    pub(crate) fn checkpoint(&self) -> OwnershipPendingCheckpoint {
        OwnershipPendingCheckpoint {
            query: self.ticket.query,
            binding: self.ticket.binding.generation(),
            player_key: self.ticket.keys.player,
            entity_key: self.ticket.keys.entity,
            player_stamp: self
                .player_expected
                .as_ref()
                .map(OwnedPlayerExpectation::stamp),
            entity_stamp: self
                .entity_expected
                .as_ref()
                .map(OwnedEntityExpectation::stamp),
            kind: self.kind,
            before_player: self.before_player.clone(),
            before_entity: self.before_entity.clone(),
            after_player: self.after_player_record.clone(),
            after_entity: self.after_entity_record.clone(),
            status: self.status,
        }
    }
}

fn kind_code(kind: PendingKind) -> u8 {
    match kind {
        PendingKind::Bootstrap => 0,
        PendingKind::Autosave => 1,
        PendingKind::Click => 2,
        PendingKind::Drop => 3,
        PendingKind::Pickup => 4,
        PendingKind::SpawnItem => 5,
    }
}
fn status_code(status: OwnershipPendingStatus) -> u8 {
    match status {
        OwnershipPendingStatus::Prepared => 0,
        OwnershipPendingStatus::InFlight => 1,
        OwnershipPendingStatus::Uncertain => 2,
        OwnershipPendingStatus::NeedsRefresh => 3,
        OwnershipPendingStatus::Quarantined => 4,
    }
}

fn player_frame(out: &mut Vec<u8>, record: Option<&PlayerRecord>) {
    out.push(u8::from(record.is_some()));
    if let Some(r) = record {
        out.extend_from_slice(&r.schema_version().get().to_be_bytes());
        out.push(r.game_mode().as_id());
        out.extend_from_slice(&(r.data().len() as u32).to_be_bytes());
        out.extend_from_slice(r.data());
    }
}
fn entity_frame(out: &mut Vec<u8>, record: Option<&PersistentEntityRecord>) {
    out.push(u8::from(record.is_some()));
    if let Some(r) = record {
        out.extend_from_slice(&r.record().schema_version().get().to_be_bytes());
        out.extend_from_slice(&r.chunk().x().to_be_bytes());
        out.extend_from_slice(&r.chunk().z().to_be_bytes());
        out.extend_from_slice(&(r.record().data().len() as u32).to_be_bytes());
        out.extend_from_slice(r.record().data());
    }
}

impl OwnershipPendingCheckpoint {
    pub(crate) fn validate(&self) -> Result<(), PlayerOwnershipError> {
        let player = self.player_key.is_some();
        let entity = self.entity_key.is_some();
        let shape = match self.kind {
            PendingKind::Bootstrap => {
                player
                    && !entity
                    && self.before_player.is_none()
                    && self.player_stamp.is_some_and(OwnershipStamp::is_fresh)
            }
            PendingKind::Autosave | PendingKind::Click => {
                player && !entity && self.before_player.is_some()
            }
            PendingKind::Drop => {
                player
                    && entity
                    && self.before_player.is_some()
                    && self.before_entity.is_none()
                    && self.entity_stamp.is_some_and(OwnershipStamp::is_fresh)
                    && self.after_entity.is_some()
            }
            PendingKind::Pickup => {
                player && entity && self.before_player.is_some() && self.before_entity.is_some()
            }
            PendingKind::SpawnItem => {
                !player
                    && entity
                    && self.before_entity.is_none()
                    && self.entity_stamp.is_some_and(OwnershipStamp::is_fresh)
                    && self.after_entity.is_some()
            }
        };
        if !shape
            || player != self.player_stamp.is_some()
            || entity != self.entity_stamp.is_some()
            || player != self.after_player.is_some()
        {
            return Err(PlayerOwnershipError::Malformed("checkpoint resource shape"));
        }
        if self.before_player.is_some() != self.player_stamp.is_some_and(OwnershipStamp::is_live)
            || self.before_entity.is_some()
                != self.entity_stamp.is_some_and(OwnershipStamp::is_live)
        {
            return Err(PlayerOwnershipError::Malformed("checkpoint before stamp"));
        }
        if self
            .player_key
            .is_some_and(|k| k.scope() != self.binding.key())
            || self.entity_key.is_some_and(|k| {
                k.world() != self.binding.key().world()
                    || k.dimension() != self.binding.key().dimension()
            })
        {
            return Err(PlayerOwnershipError::ScopeMismatch);
        }
        let before = self.before_player.as_ref().map_or(0, |r| r.data().len())
            + self
                .before_entity
                .as_ref()
                .map_or(0, |r| r.record().data().len());
        let after = self.after_player.as_ref().map_or(0, |r| r.data().len())
            + self
                .after_entity
                .as_ref()
                .map_or(0, |r| r.record().data().len());
        if 661_usize
            .checked_add(before.saturating_mul(2))
            .and_then(|n| n.checked_add(after.saturating_mul(5)))
            .is_none_or(|n| n > super::MAX_PLAYER_OWNERSHIP_PENDING_BYTES)
        {
            return Err(PlayerOwnershipError::Capacity("checkpoint charge"));
        }
        for record in self.before_player.iter().chain(&self.after_player) {
            NativePlayerSnapshot::try_from_record(record)?;
        }
        for record in self.before_entity.iter().chain(&self.after_entity) {
            crate::decode_item_entity(
                self.entity_key
                    .ok_or(PlayerOwnershipError::Malformed("checkpoint entity key"))?,
                record,
            )?;
        }
        Ok(())
    }

    /// Encodes bounded version-1 data. This grants no retry or completion authority.
    pub fn try_to_bytes(&self) -> Result<Vec<u8>, PlayerOwnershipError> {
        self.validate()?;
        let total = 256
            + self.before_player.as_ref().map_or(0, |r| r.data().len())
            + self.after_player.as_ref().map_or(0, |r| r.data().len())
            + self
                .before_entity
                .as_ref()
                .map_or(0, |r| r.record().data().len())
            + self
                .after_entity
                .as_ref()
                .map_or(0, |r| r.record().data().len());
        let mut out = Vec::new();
        out.try_reserve_exact(total)
            .map_err(|_| PlayerOwnershipError::AllocationFailed)?;
        out.extend_from_slice(b"FCPQ");
        out.extend_from_slice(&1_u16.to_be_bytes());
        out.push(kind_code(self.kind));
        out.push(status_code(self.status));
        out.extend_from_slice(&self.binding.key().world().get().to_be_bytes());
        out.extend_from_slice(&self.binding.key().dimension().get().to_be_bytes());
        out.extend_from_slice(&self.binding.identity().encode());
        out.extend_from_slice(&self.query.id().to_bytes());
        out.extend_from_slice(&self.query.identity().to_bytes());
        out.push(u8::from(self.player_key.is_some()));
        if let Some(key) = self.player_key {
            out.extend_from_slice(key.id().as_uuid().as_bytes());
            out.extend_from_slice(
                &self
                    .player_stamp
                    .ok_or(PlayerOwnershipError::Malformed("checkpoint stamp"))?
                    .encode(),
            );
        }
        out.push(u8::from(self.entity_key.is_some()));
        if let Some(key) = self.entity_key {
            out.extend_from_slice(&key.uuid().to_bytes());
            out.extend_from_slice(
                &self
                    .entity_stamp
                    .ok_or(PlayerOwnershipError::Malformed("checkpoint stamp"))?
                    .encode(),
            );
        }
        player_frame(&mut out, self.before_player.as_ref());
        entity_frame(&mut out, self.before_entity.as_ref());
        player_frame(&mut out, self.after_player.as_ref());
        entity_frame(&mut out, self.after_entity.as_ref());
        Ok(out)
    }

    /// Checks fixed grammar, complete frame extents, native codecs and charge limits.
    /// Decoded data cannot recreate a live worker, expectation or shard handle.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, PlayerOwnershipError> {
        if bytes.len() > super::MAX_PLAYER_OWNERSHIP_PENDING_BYTES {
            return Err(PlayerOwnershipError::Capacity("checkpoint bytes"));
        }
        let mut r = Reader { bytes, at: 0 };
        if r.take(4)? != b"FCPQ" || u16::from_be_bytes(r.fixed()?) != 1 {
            return Err(PlayerOwnershipError::Malformed("checkpoint version"));
        }
        let kind = match r.byte()? {
            0 => PendingKind::Bootstrap,
            1 => PendingKind::Autosave,
            2 => PendingKind::Click,
            3 => PendingKind::Drop,
            4 => PendingKind::Pickup,
            5 => PendingKind::SpawnItem,
            _ => return Err(PlayerOwnershipError::Malformed("checkpoint kind")),
        };
        let status = match r.byte()? {
            0 => OwnershipPendingStatus::Prepared,
            1 => OwnershipPendingStatus::InFlight,
            2 => OwnershipPendingStatus::Uncertain,
            3 => OwnershipPendingStatus::NeedsRefresh,
            4 => OwnershipPendingStatus::Quarantined,
            _ => return Err(PlayerOwnershipError::Malformed("checkpoint phase")),
        };
        let scope = GenerationKey::new(
            WorldId::new(u32::from_be_bytes(r.fixed()?)),
            DimensionId::new(u32::from_be_bytes(r.fixed()?)),
        );
        let identity = GenerationIdentity::decode(r.take(GENERATION_IDENTITY_LEN)?)
            .map_err(|_| PlayerOwnershipError::Malformed("checkpoint generation"))?;
        let query = OwnershipReceiptQuery::from_bytes(
            OwnershipTransferId::from_bytes(r.fixed()?),
            r.fixed()?,
        );
        let (player_key, player_stamp) = if r.presence()? {
            let id = PlayerId::from_uuid(EntityUuid::from_bytes(r.fixed()?).as_uuid());
            (
                Some(ScopedPlayerKey::new(scope, id)),
                Some(OwnershipStamp::decode(r.take(9)?)?),
            )
        } else {
            (None, None)
        };
        let (entity_key, entity_stamp) = if r.presence()? {
            let id = EntityUuid::from_bytes(r.fixed()?);
            (
                Some(PersistentEntityKey::new(
                    scope.world(),
                    scope.dimension(),
                    id,
                )),
                Some(OwnershipStamp::decode(r.take(9)?)?),
            )
        } else {
            (None, None)
        };
        // Borrowed frame descriptors check all extents and aggregate charge
        // before allocating any nested player/entity/component representation.
        let before_player = r.player()?;
        let before_entity = r.entity()?;
        let after_player = r.player()?;
        let after_entity = r.entity()?;
        if r.at != bytes.len() {
            return Err(PlayerOwnershipError::Malformed("checkpoint trailing bytes"));
        }
        let before = before_player.as_ref().map_or(0, |r| r.2.len())
            + before_entity.as_ref().map_or(0, |r| r.2.len());
        let after = after_player.as_ref().map_or(0, |r| r.2.len())
            + after_entity.as_ref().map_or(0, |r| r.2.len());
        if 661 + before * 2 + after * 5 > super::MAX_PLAYER_OWNERSHIP_PENDING_BYTES {
            return Err(PlayerOwnershipError::Capacity("checkpoint charge"));
        }
        let checkpoint = Self {
            query,
            binding: crate::GenerationBinding::new(scope, identity),
            player_key,
            entity_key,
            player_stamp,
            entity_stamp,
            kind,
            status,
            before_player: owned_player(before_player)?,
            before_entity: owned_entity(before_entity)?,
            after_player: owned_player(after_player)?,
            after_entity: owned_entity(after_entity)?,
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }
}

type PlayerFrame<'a> = Option<(SchemaVersion, GameMode, &'a [u8])>;
type EntityFrame<'a> = Option<(SchemaVersion, ChunkPos, &'a [u8])>;
fn owned_player(frame: PlayerFrame<'_>) -> Result<Option<PlayerRecord>, PlayerOwnershipError> {
    frame
        .map(|(schema, mode, bytes)| {
            PlayerRecord::new(schema, mode, bytes.to_vec())
                .map_err(|e| PlayerOwnershipError::Storage(e.into()))
        })
        .transpose()
}
fn owned_entity(
    frame: EntityFrame<'_>,
) -> Result<Option<PersistentEntityRecord>, PlayerOwnershipError> {
    frame
        .map(|(schema, chunk, bytes)| {
            let record = EntityRecord::new(schema, bytes.to_vec())
                .map_err(|e| PlayerOwnershipError::Storage(e.into()))?;
            PersistentEntityRecord::new(chunk, record)
                .map_err(|e| PlayerOwnershipError::Storage(e.into()))
        })
        .transpose()
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], PlayerOwnershipError> {
        let end = self
            .at
            .checked_add(count)
            .ok_or(PlayerOwnershipError::Malformed("checkpoint length"))?;
        let b = self
            .bytes
            .get(self.at..end)
            .ok_or(PlayerOwnershipError::Malformed("checkpoint truncated"))?;
        self.at = end;
        Ok(b)
    }
    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], PlayerOwnershipError> {
        self.take(N)?
            .try_into()
            .map_err(|_| PlayerOwnershipError::Malformed("checkpoint truncated"))
    }
    fn byte(&mut self) -> Result<u8, PlayerOwnershipError> {
        Ok(self.fixed::<1>()?[0])
    }
    fn presence(&mut self) -> Result<bool, PlayerOwnershipError> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(PlayerOwnershipError::Malformed("checkpoint presence")),
        }
    }
    fn body(&mut self, cap: usize) -> Result<&'a [u8], PlayerOwnershipError> {
        let len = u32::from_be_bytes(self.fixed()?) as usize;
        if len > cap {
            return Err(PlayerOwnershipError::Capacity("checkpoint record"));
        }
        self.take(len)
    }
    fn player(&mut self) -> Result<PlayerFrame<'a>, PlayerOwnershipError> {
        if !self.presence()? {
            return Ok(None);
        }
        let schema = SchemaVersion::new(u32::from_be_bytes(self.fixed()?));
        let mode = GameMode::from_id(self.byte()?)
            .ok_or(PlayerOwnershipError::Malformed("checkpoint game mode"))?;
        Ok(Some((
            schema,
            mode,
            self.body(ferrumc_storage::MAX_PLAYER_DATA_LEN)?,
        )))
    }
    fn entity(&mut self) -> Result<EntityFrame<'a>, PlayerOwnershipError> {
        if !self.presence()? {
            return Ok(None);
        }
        let schema = SchemaVersion::new(u32::from_be_bytes(self.fixed()?));
        let chunk = ChunkPos::new(
            i32::from_be_bytes(self.fixed()?),
            i32::from_be_bytes(self.fixed()?),
        );
        Ok(Some((
            schema,
            chunk,
            self.body(ferrumc_storage::MAX_ENTITY_DATA_LEN)?,
        )))
    }
}

/// Receipt-positive fresh readonly admissions after restart, without old handles.
#[derive(Debug)]
pub struct OwnershipCheckpointResolution {
    player: Option<OwnedPlayerRead>,
    entity: Option<OwnedEntityRead>,
}
impl OwnershipCheckpointResolution {
    /// Moves real current admissions to the normal restore path. Entity tombstones
    /// have no spawn admission. This object does not replay historical publications.
    pub fn into_reads(self) -> (Option<OwnedPlayerRead>, Option<OwnedEntityRead>) {
        (self.player, self.entity)
    }
}

impl OwnershipStoreBinding {
    /// Clarifies checked data using a real receipt and exact current postimages.
    /// `None` explicitly refuses recovery; no boolean or Weak liveness is accepted
    /// as proof that a previous worker stopped.
    pub async fn clarify_checkpoint(
        &self,
        store: &Arc<dyn PlayerEntityOwnershipStore>,
        checkpoint: &OwnershipPendingCheckpoint,
    ) -> Result<OwnershipCheckpointResolution, PlayerOwnershipError> {
        self.check_store(store)?;
        checkpoint.validate()?;
        if self.generation() != checkpoint.binding {
            return Err(PlayerOwnershipError::ScopeMismatch);
        }
        let receipt = store
            .load_ownership_receipt(checkpoint.query)
            .await?
            .ok_or(PlayerOwnershipError::UnprovenWorkerLifetime)?;
        if receipt.query() != checkpoint.query
            || receipt.scope() != checkpoint.binding.key()
            || receipt.generation_identity() != checkpoint.binding.identity()
            || receipt
                .player()
                .map(|(id, before, _)| (ScopedPlayerKey::new(receipt.scope(), id), before))
                != checkpoint.player_key.zip(checkpoint.player_stamp)
            || receipt.entity().map(|(id, before, _)| {
                (
                    PersistentEntityKey::new(
                        receipt.scope().world(),
                        receipt.scope().dimension(),
                        id,
                    ),
                    before,
                )
            }) != checkpoint.entity_key.zip(checkpoint.entity_stamp)
            || receipt.before_entity_chunk()
                != checkpoint
                    .before_entity
                    .as_ref()
                    .map(PersistentEntityRecord::chunk)
            || receipt.after_entity_chunk()
                != checkpoint
                    .after_entity
                    .as_ref()
                    .map(PersistentEntityRecord::chunk)
        {
            return Err(PlayerOwnershipError::CompletionMismatch);
        }
        check_receipt_epochs(
            &receipt,
            checkpoint.binding,
            checkpoint.before_entity.as_ref(),
            checkpoint.after_entity.as_ref(),
        )?;
        let (player, entity) = match (checkpoint.player_key, checkpoint.entity_key) {
            (Some(p), Some(e)) => {
                let pair = self.read_pair(store, p.id(), e.uuid()).await?;
                let (player, entity) = pair.snapshot.into_parts();
                (
                    Some(OwnedPlayerRead {
                        binding: self.clone(),
                        snapshot: player,
                    }),
                    Some(OwnedEntityRead {
                        binding: self.clone(),
                        snapshot: entity,
                    }),
                )
            }
            (Some(p), None) => (Some(self.read_player(store, p.id()).await?), None),
            (None, Some(e)) => (None, Some(self.read_entity(store, e.uuid()).await?)),
            (None, None) => return Err(PlayerOwnershipError::Malformed("empty checkpoint")),
        };
        if let Some(read) = &player {
            let after = receipt
                .player()
                .ok_or(PlayerOwnershipError::CompletionMismatch)?
                .2;
            if read.snapshot.stamp() != after
                || !equal_player(read.snapshot.record(), checkpoint.after_player.as_ref())
            {
                return Err(PlayerOwnershipError::Superseded);
            }
        }
        if let Some(read) = &entity {
            let after = receipt
                .entity()
                .ok_or(PlayerOwnershipError::CompletionMismatch)?
                .2;
            if read.snapshot.stamp() != after
                || !equal_entity(read.snapshot.record(), checkpoint.after_entity.as_ref())
            {
                return Err(PlayerOwnershipError::Superseded);
            }
        }
        let entity = entity.filter(|_| checkpoint.after_entity.is_some());
        Ok(OwnershipCheckpointResolution { player, entity })
    }
}

fn equal_player(a: Option<&PlayerRecord>, b: Option<&PlayerRecord>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            a.schema_version() == b.schema_version()
                && a.game_mode() == b.game_mode()
                && a.data() == b.data()
        }
        (None, None) => true,
        _ => false,
    }
}
fn equal_entity(a: Option<&PersistentEntityRecord>, b: Option<&PersistentEntityRecord>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            a.chunk() == b.chunk()
                && a.record().schema_version() == b.record().schema_version()
                && a.record().data() == b.record().data()
        }
        (None, None) => true,
        _ => false,
    }
}
