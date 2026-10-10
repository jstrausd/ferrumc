//! Exact Format5 byte grammar; parsing borrows the body until admission succeeds.

use ferrumc_core::{DimensionId, EntityUuid, WorldId};
use ferrumc_math::ChunkPos;

use super::{PersistentEntityKey, PersistentEntityRecord};
use crate::{ChunkKey, EntityRecord, SchemaVersion, StorageError, MAX_ENTITY_DATA_LEN};

pub(crate) const ENTITY: &str = "ferrumc:entity";
pub(crate) const LOCATOR: &str = "ferrumc:entity_locator";
pub(crate) const INDEX: &str = "ferrumc:entity_chunk";
pub(crate) const EPOCH: &str = "ferrumc:entity_chunk_epoch";
pub(crate) const HEADER_LEN: usize = 22;
pub(crate) const KEY_LEN: usize = 24;

pub(crate) fn corrupt(table: &'static str, reason: &'static str) -> StorageError {
    StorageError::PersistentEntityCorruption { table, reason }
}

fn array<const N: usize>(bytes: &[u8], table: &'static str) -> Result<[u8; N], StorageError> {
    bytes
        .try_into()
        .map_err(|_| corrupt(table, "invalid fixed-width length"))
}

pub(crate) fn key_bytes(key: PersistentEntityKey) -> [u8; KEY_LEN] {
    let mut bytes = [0; KEY_LEN];
    bytes[..4].copy_from_slice(&key.world().get().to_be_bytes());
    bytes[4..8].copy_from_slice(&key.dimension().get().to_be_bytes());
    bytes[8..].copy_from_slice(&key.uuid().to_bytes());
    bytes
}

pub(crate) fn decode_key(bytes: &[u8]) -> Result<PersistentEntityKey, StorageError> {
    let bytes = array::<KEY_LEN>(bytes, ENTITY)?;
    Ok(PersistentEntityKey::new(
        WorldId::new(u32::from_be_bytes(array(&bytes[..4], ENTITY)?)),
        DimensionId::new(u32::from_be_bytes(array(&bytes[4..8], ENTITY)?)),
        EntityUuid::from_bytes(array(&bytes[8..], ENTITY)?),
    ))
}

pub(crate) fn locator_key(key: PersistentEntityKey) -> [u8; 20] {
    let mut bytes = [0; 20];
    bytes[..4].copy_from_slice(&key.world().get().to_be_bytes());
    bytes[4..].copy_from_slice(&key.uuid().to_bytes());
    bytes
}

pub(crate) fn decode_locator_key(bytes: &[u8]) -> Result<(WorldId, EntityUuid), StorageError> {
    let bytes = array::<20>(bytes, LOCATOR)?;
    Ok((
        WorldId::new(u32::from_be_bytes(array(&bytes[..4], LOCATOR)?)),
        EntityUuid::from_bytes(array(&bytes[4..], LOCATOR)?),
    ))
}

pub(crate) fn locator_value(key: PersistentEntityKey, chunk: ChunkPos) -> [u8; 12] {
    let mut bytes = [0; 12];
    bytes[..4].copy_from_slice(&key.dimension().get().to_be_bytes());
    bytes[4..8].copy_from_slice(&chunk.x().to_be_bytes());
    bytes[8..].copy_from_slice(&chunk.z().to_be_bytes());
    bytes
}

pub(crate) fn decode_locator(bytes: &[u8]) -> Result<(DimensionId, ChunkPos), StorageError> {
    let bytes = array::<12>(bytes, LOCATOR)?;
    let dimension = DimensionId::new(u32::from_be_bytes(array(&bytes[..4], LOCATOR)?));
    let chunk = ChunkPos::new(
        i32::from_be_bytes(array(&bytes[4..8], LOCATOR)?),
        i32::from_be_bytes(array(&bytes[8..], LOCATOR)?),
    );
    crate::record::validate_chunk_position(chunk)?;
    Ok((dimension, chunk))
}

pub(crate) fn index_key(key: PersistentEntityKey, chunk: ChunkPos) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[..16].copy_from_slice(&crate::codec::chunk_key_bytes(ChunkKey::new(
        key.world(),
        key.dimension(),
        chunk,
    )));
    bytes[16..].copy_from_slice(&key.uuid().to_bytes());
    bytes
}

pub(crate) fn decode_chunk(bytes: &[u8], table: &'static str) -> Result<ChunkKey, StorageError> {
    let bytes = array::<16>(bytes, table)?;
    let chunk = ChunkPos::new(
        i32::from_be_bytes(array(&bytes[8..12], table)?),
        i32::from_be_bytes(array(&bytes[12..], table)?),
    );
    crate::record::validate_chunk_position(chunk)?;
    Ok(ChunkKey::new(
        WorldId::new(u32::from_be_bytes(array(&bytes[..4], table)?)),
        DimensionId::new(u32::from_be_bytes(array(&bytes[4..8], table)?)),
        chunk,
    ))
}

pub(crate) fn decode_index(
    bytes: &[u8],
    value: &[u8],
) -> Result<(ChunkKey, EntityUuid), StorageError> {
    let bytes = array::<32>(bytes, INDEX)?;
    if !value.is_empty() {
        return Err(corrupt(INDEX, "index value must be empty"));
    }
    Ok((
        decode_chunk(&bytes[..16], INDEX)?,
        EntityUuid::from_bytes(array(&bytes[16..], INDEX)?),
    ))
}

pub(crate) fn decode_epoch(bytes: &[u8]) -> Result<u64, StorageError> {
    let value = u64::from_be_bytes(array(bytes, EPOCH)?);
    if value == 0 {
        return Err(corrupt(EPOCH, "persisted epoch must be positive"));
    }
    Ok(value)
}

pub(crate) struct Envelope<'a> {
    pub(crate) chunk: ChunkPos,
    schema: SchemaVersion,
    pub(crate) body: &'a [u8],
}
impl Envelope<'_> {
    pub(crate) fn owned(&self) -> Result<PersistentEntityRecord, StorageError> {
        PersistentEntityRecord::new(
            self.chunk,
            EntityRecord::new(self.schema, self.body.to_vec())?,
        )
    }
}

pub(crate) fn envelope(bytes: &[u8]) -> Result<Envelope<'_>, StorageError> {
    if bytes.len() < HEADER_LEN {
        return Err(corrupt(ENTITY, "truncated envelope"));
    }
    if &bytes[..4] != b"FCUE" || u16::from_be_bytes(array(&bytes[4..6], ENTITY)?) != 1 {
        return Err(corrupt(ENTITY, "unsupported envelope magic/schema"));
    }
    let chunk = ChunkPos::new(
        i32::from_be_bytes(array(&bytes[6..10], ENTITY)?),
        i32::from_be_bytes(array(&bytes[10..14], ENTITY)?),
    );
    crate::record::validate_chunk_position(chunk)?;
    let schema = SchemaVersion::new(u32::from_be_bytes(array(&bytes[14..18], ENTITY)?));
    let len = u32::from_be_bytes(array(&bytes[18..22], ENTITY)?) as usize;
    if len > MAX_ENTITY_DATA_LEN {
        return Err(StorageError::RecordTooLarge {
            len,
            max: MAX_ENTITY_DATA_LEN,
        });
    }
    if bytes.len() != HEADER_LEN + len {
        return Err(corrupt(ENTITY, "payload length or trailing bytes"));
    }
    Ok(Envelope {
        chunk,
        schema,
        body: &bytes[HEADER_LEN..],
    })
}

pub(crate) fn encode(record: &PersistentEntityRecord) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER_LEN + record.record().data().len());
    bytes.extend_from_slice(b"FCUE");
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&record.chunk().x().to_be_bytes());
    bytes.extend_from_slice(&record.chunk().z().to_be_bytes());
    bytes.extend_from_slice(&record.record().schema_version().get().to_be_bytes());
    // EntityRecord admission bounds this length below u32MAX.
    bytes.extend_from_slice(&(record.record().data().len() as u32).to_be_bytes());
    bytes.extend_from_slice(record.record().data());
    bytes
}

pub(crate) fn prefix_upper(mut prefix: [u8; 16]) -> Option<[u8; 16]> {
    for index in (0..prefix.len()).rev() {
        if prefix[index] != u8::MAX {
            prefix[index] += 1;
            prefix[index + 1..].fill(0);
            return Some(prefix);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> PersistentEntityRecord {
        PersistentEntityRecord::new(
            ChunkPos::new(-1, 2),
            EntityRecord::new(SchemaVersion::new(u32::MAX), vec![1, 2]).unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn all_fixed_width_codecs_preserve_extrema_and_canonical_big_endian_bytes() {
        let key = PersistentEntityKey::new(
            WorldId::new(0x0102_0304),
            DimensionId::new(u32::MAX),
            EntityUuid::from_bytes([255; 16]),
        );
        assert_eq!(&key_bytes(key)[..8], &[1, 2, 3, 4, 255, 255, 255, 255]);
        assert_eq!(decode_key(&key_bytes(key)).unwrap(), key);
        assert_eq!(
            decode_locator_key(&locator_key(key)).unwrap(),
            (key.world(), key.uuid())
        );
        assert_eq!(
            decode_locator(&locator_value(key, record().chunk())).unwrap(),
            (key.dimension(), record().chunk())
        );
        let expected = ChunkKey::new(key.world(), key.dimension(), record().chunk());
        assert_eq!(
            decode_chunk(&crate::codec::chunk_key_bytes(expected), EPOCH).unwrap(),
            expected
        );
        assert_eq!(
            decode_index(&index_key(key, record().chunk()), &[]).unwrap(),
            (expected, key.uuid())
        );
        assert_eq!(decode_epoch(&u64::MAX.to_be_bytes()).unwrap(), u64::MAX);
        assert!(decode_epoch(&[0; 8]).is_err());
        assert_eq!(prefix_upper([255; 16]), None);
        let mut lower = [0; 16];
        lower[15] = 255;
        let mut upper = [0; 16];
        upper[14] = 1;
        assert_eq!(prefix_upper(lower), Some(upper));
    }
    #[test]
    fn every_fixed_width_parser_refuses_short_long_and_trailing_bytes() {
        for len in [0, 1, 19, 20, 23, 25, 31, 32, 33, 100] {
            if len != 24 {
                assert!(decode_key(&vec![0; len]).is_err());
            }
            if len != 20 {
                assert!(decode_locator_key(&vec![0; len]).is_err());
            }
            if len != 12 {
                assert!(decode_locator(&vec![0; len]).is_err());
            }
            if len != 16 {
                assert!(decode_chunk(&vec![0; len], EPOCH).is_err());
            }
            if len != 32 {
                assert!(decode_index(&vec![0; len], &[]).is_err());
            }
            if len != 8 {
                assert!(decode_epoch(&vec![0; len]).is_err());
            }
        }
        assert!(decode_index(&[0; 32], &[0]).is_err());
        let mut bad = [0; 12];
        bad[4..8].copy_from_slice(&i32::MAX.to_be_bytes());
        assert!(decode_locator(&bad).is_err());
    }
    #[test]
    fn envelope_borrows_exact_bounded_body_and_rejects_all_grammar_failures() {
        let bytes = encode(&record());
        assert_eq!(&bytes[..6], b"FCUE\0\x01");
        assert_eq!(&bytes[6..10], &(-1i32).to_be_bytes());
        assert_eq!(envelope(&bytes).unwrap().owned().unwrap(), record());
        for len in 0..HEADER_LEN {
            assert!(envelope(&bytes[..len]).is_err());
        }
        for field in [0, 4, 5, 18, 19, 20, 21] {
            let mut malformed = bytes.clone();
            malformed[field] ^= 0xff;
            assert!(envelope(&malformed).is_err(), "field {field}");
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(envelope(&trailing).is_err());
        let mut short = bytes.clone();
        short.pop();
        assert!(envelope(&short).is_err());
        let mut outside = bytes.clone();
        outside[6..10].copy_from_slice(&i32::MIN.to_be_bytes());
        assert!(envelope(&outside).is_err());
        let max = PersistentEntityRecord::new(
            ChunkPos::new(-134_217_728, 134_217_727),
            EntityRecord::new(SchemaVersion::new(0), vec![255; MAX_ENTITY_DATA_LEN]).unwrap(),
        )
        .unwrap();
        assert_eq!(envelope(&encode(&max)).unwrap().owned().unwrap(), max);
        let mut excess = encode(&max);
        excess[18..22].copy_from_slice(&((MAX_ENTITY_DATA_LEN + 1) as u32).to_be_bytes());
        excess.push(1);
        assert!(matches!(
            envelope(&excess),
            Err(StorageError::RecordTooLarge { .. })
        ));
    }
}
