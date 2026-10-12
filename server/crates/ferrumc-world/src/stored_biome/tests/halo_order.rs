//! Carried GEN-05.8 runtime condition R-A1 (GEN-05.9 review A6): the documented
//! two-phase halo refusal order of the surface resolver is pinned by a test.
//! Production behaviour of `stored_biome` is unchanged.

use super::*;

fn small() -> StoredBiomeIdentity {
    control_identity(BiomeQuartGeometry::new(0, 16).unwrap())
}

/// An edge centre with an additionally missing in-domain slot still refuses
/// with the first out-of-domain key in slot order (every domain first), never
/// `MissingChunk`; the missing in-domain slot stays a sparse lookup refusal.
/// A single-pass halo (per slot: domain, then presence) fails this test.
#[test]
fn edge_surface_halo_reports_every_domain_before_any_missing_in_domain_slot() {
    let identity = small();
    for (center, drop, expected) in [
        // Slot 0 = (-1,-1) is in domain and dropped; slot 6 = (+1,-1) leaves X.
        (ChunkPos::new(1_874_999, 0), 0, ChunkPos::new(1_875_000, -1)),
        // Slot 8 = (+1,+1) dropped; slot 0 = (-1,-1) leaves X.
        (
            ChunkPos::new(-1_875_000, 0),
            8,
            ChunkPos::new(-1_875_001, -1),
        ),
        // Z+ edge: slot 0 dropped; slot 2 = (-1,+1) leaves Z.
        (ChunkPos::new(0, 1_874_999), 0, ChunkPos::new(-1, 1_875_000)),
        // Z- edge: slot 8 dropped; slot 0 = (-1,-1) leaves Z.
        (
            ChunkPos::new(0, -1_875_000),
            8,
            ChunkPos::new(-1, -1_875_001),
        ),
    ] {
        let controls: Vec<_> = (0..MAX_STORED_BIOME_CHUNKS)
            .filter(|slot| *slot != drop)
            .map(|slot| slot_key(center, slot).unwrap())
            .filter(|key| admit_chunk(*key).is_ok())
            .map(|key| control(identity, key))
            .collect();
        assert_eq!(controls.len(), 5);
        let refs: Vec<_> = controls.iter().collect();
        let core = admit_controls(identity, center, &refs, 10_000).unwrap();
        let error = complete_halo(&core).unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::CoordinateDomain);
        assert_eq!(error.chunk(), Some(expected));
        let key = slot_key(center, drop).unwrap();
        let error = core
            .resolve(StoredBiomeQuartPos::new(key.x() * 4, 0, key.z() * 4))
            .unwrap_err();
        assert_eq!(error.kind(), StoredBiomeErrorKind::MissingChunk);
        assert_eq!(error.chunk(), Some(key));
    }
}

/// With every domain admitted, the first missing slot in slot order is named.
#[test]
fn interior_surface_halo_reports_the_first_missing_slot_in_slot_order() {
    let identity = small();
    let center = ChunkPos::new(10, -10);
    let controls: Vec<_> = [1, 2, 4, 5, 7, 8]
        .into_iter()
        .map(|slot| control(identity, slot_key(center, slot).unwrap()))
        .collect();
    let refs: Vec<_> = controls.iter().collect();
    let core = admit_controls(identity, center, &refs, 10_000).unwrap();
    let error = complete_halo(&core).unwrap_err();
    assert_eq!(error.kind(), StoredBiomeErrorKind::MissingChunk);
    assert_eq!(error.chunk(), Some(slot_key(center, 0).unwrap()));
}
