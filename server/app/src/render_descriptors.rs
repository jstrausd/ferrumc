//! Default client render descriptors derived only from current resident states.
//!
//! No persistent block entity, inventory, interaction or animation authority is
//! fabricated. Custom patterns/profiles/sherds and gameplay remain out of scope.
use ferrumc_math::LocalBlockPos;
use ferrumc_nbt::{NbtCompound, NbtTag};
use ferrumc_proto::generated::play::ChunkBlockEntity;
use ferrumc_world::{BlockStateId, Chunk, MAX_BLOCK_ENTITIES};

/// Append default-render descriptors after the existing typed container entries.
///
/// The shared 4,096 descriptor cap is preflighted without changing the caller's
/// entries. At most 98,304 cells are inspected once, with nine numeric state
/// intervals and no registry-name lookups. Empty sections need no cell scan.
/// Temporary wire-field capacity is at most 4,096 entries, allocated only after a
/// matching state occurs; no mutable global, cache or per-tick work is added.
///
/// Anonymous empty compound NBT encodes as two bytes (10,0); Vanilla's nullable
/// empty update tag encodes as one byte (0). Both establish a fresh client entity
/// with default fields, but this does not claim identical update tags or support
/// for custom banner patterns, skull profiles or decorated-pot sherds.
pub(crate) fn append_state_render_entities(
    chunk: &Chunk,
    entries: &mut Vec<ChunkBlockEntity>,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        entries.len() <= MAX_BLOCK_ENTITIES,
        "chunk render descriptor cap exceeded"
    );
    let available = MAX_BLOCK_ENTITIES - entries.len();
    let mut proposed: Vec<(u8, i16, i32)> = Vec::new();
    for (section_index, section) in chunk.sections().iter().enumerate() {
        if section.is_empty() {
            continue;
        }
        let base_y = ferrumc_registry::dimension::MIN_Y + i32::try_from(section_index)? * 16;
        for local_y in 0_u8..16 {
            let y = i16::try_from(base_y + i32::from(local_y))?;
            for local_z in 0_u8..16 {
                for local_x in 0_u8..16 {
                    let local = LocalBlockPos::new(local_x, local_y, local_z).ok_or_else(|| {
                        anyhow::anyhow!("invalid bounded local render coordinate")
                    })?;
                    let Some(kind) = default_render_type(section.get(local)) else {
                        continue;
                    };
                    anyhow::ensure!(
                        proposed.len() < available,
                        "chunk render descriptor cap exceeded"
                    );
                    if proposed.is_empty() {
                        proposed.try_reserve_exact(available)?;
                    }
                    proposed.push(((local_x << 4) | local_z, y, kind));
                }
            }
        }
    }
    entries.try_reserve_exact(proposed.len())?;
    for (packed_xz, y, kind) in proposed {
        entries.push(ChunkBlockEntity::new(
            packed_xz,
            y,
            kind,
            NbtTag::Compound(NbtCompound::new()),
        ));
    }
    Ok(())
}

/// Exact 1.21.8 registry intervals, independently checked against all 84 names and
/// all 1,017 states. Existing signs/chests/furnaces are deliberately excluded.
/// Unknown or unsupported states never acquire a guessed descriptor.
const fn default_render_type(state: BlockStateId) -> Option<i32> {
    match state.as_u32() {
        1731..=1986 => Some(25),   // bed
        8173 => Some(13),          // enchanting_table
        8297..=8304 => Some(3),    // ender_chest
        9636..=9915 => Some(16),   // skull
        11648..=11967 => Some(20), // banner
        13585..=13686 => Some(24), // shulker_box
        13997..=13998 => Some(26), // conduit
        19526..=19557 => Some(31), // bell
        27666..=27681 => Some(41), // decorated_pot
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use ferrumc_codec::BoundedReader;
    use ferrumc_math::{BlockPos, ChunkPos};
    use ferrumc_registry::block_state::{
        block_default_state, block_metadata, compute_state_id, state_id_to_block_name,
    };
    use ferrumc_world::{BlockEntity, ChestInventory};
    use std::collections::{BTreeMap, BTreeSet};

    // Independent names/types from actual Paper BlockEntityType registrations
    // and original 1.21.8 registry report; not reconstructed from the intervals.
    const EXPECTED_NAMES: &[(&str, i32)] = &[
        ("ender_chest", 3),
        ("enchanting_table", 13),
        ("skeleton_skull", 16),
        ("skeleton_wall_skull", 16),
        ("creeper_head", 16),
        ("creeper_wall_head", 16),
        ("dragon_head", 16),
        ("dragon_wall_head", 16),
        ("zombie_head", 16),
        ("zombie_wall_head", 16),
        ("wither_skeleton_skull", 16),
        ("wither_skeleton_wall_skull", 16),
        ("player_head", 16),
        ("player_wall_head", 16),
        ("piglin_head", 16),
        ("piglin_wall_head", 16),
        ("white_banner", 20),
        ("orange_banner", 20),
        ("magenta_banner", 20),
        ("light_blue_banner", 20),
        ("yellow_banner", 20),
        ("lime_banner", 20),
        ("pink_banner", 20),
        ("gray_banner", 20),
        ("light_gray_banner", 20),
        ("cyan_banner", 20),
        ("purple_banner", 20),
        ("blue_banner", 20),
        ("brown_banner", 20),
        ("green_banner", 20),
        ("red_banner", 20),
        ("black_banner", 20),
        ("white_wall_banner", 20),
        ("orange_wall_banner", 20),
        ("magenta_wall_banner", 20),
        ("light_blue_wall_banner", 20),
        ("yellow_wall_banner", 20),
        ("lime_wall_banner", 20),
        ("pink_wall_banner", 20),
        ("gray_wall_banner", 20),
        ("light_gray_wall_banner", 20),
        ("cyan_wall_banner", 20),
        ("purple_wall_banner", 20),
        ("blue_wall_banner", 20),
        ("brown_wall_banner", 20),
        ("green_wall_banner", 20),
        ("red_wall_banner", 20),
        ("black_wall_banner", 20),
        ("shulker_box", 24),
        ("black_shulker_box", 24),
        ("blue_shulker_box", 24),
        ("brown_shulker_box", 24),
        ("cyan_shulker_box", 24),
        ("gray_shulker_box", 24),
        ("green_shulker_box", 24),
        ("light_blue_shulker_box", 24),
        ("light_gray_shulker_box", 24),
        ("lime_shulker_box", 24),
        ("magenta_shulker_box", 24),
        ("orange_shulker_box", 24),
        ("pink_shulker_box", 24),
        ("purple_shulker_box", 24),
        ("red_shulker_box", 24),
        ("white_shulker_box", 24),
        ("yellow_shulker_box", 24),
        ("red_bed", 25),
        ("black_bed", 25),
        ("blue_bed", 25),
        ("brown_bed", 25),
        ("cyan_bed", 25),
        ("gray_bed", 25),
        ("green_bed", 25),
        ("light_blue_bed", 25),
        ("light_gray_bed", 25),
        ("lime_bed", 25),
        ("magenta_bed", 25),
        ("orange_bed", 25),
        ("pink_bed", 25),
        ("purple_bed", 25),
        ("white_bed", 25),
        ("yellow_bed", 25),
        ("conduit", 26),
        ("bell", 31),
        ("decorated_pot", 41),
    ];

    fn state(name: &str) -> BlockStateId {
        BlockStateId::new(block_default_state(name).expect("registered fixture state"))
    }

    fn place(chunk: &mut Chunk, position: BlockPos, name: &str) {
        chunk
            .set_block(position, state(name))
            .expect("fixture position");
    }

    fn descriptor(kind: i32) -> ChunkBlockEntity {
        ChunkBlockEntity::new(0, -64, kind, NbtTag::Compound(NbtCompound::new()))
    }

    #[test]
    fn exact84_names_and_all1017_matching_states_agree_with_real_registry() {
        assert_eq!(EXPECTED_NAMES.len(), 84);
        let mut seen = BTreeSet::new();
        for &(name, kind) in EXPECTED_NAMES {
            let metadata = block_metadata(name).expect("real metadata");
            for raw in metadata.min_state..=metadata.max_state {
                assert!(seen.insert(raw), "duplicate independent fixture state");
                assert_eq!(state_id_to_block_name(raw), Some(name));
                assert_eq!(
                    default_render_type(BlockStateId::new(raw)),
                    Some(kind),
                    "{name}/{raw}"
                );
            }
        }
        assert_eq!(seen.len(), 1017);
        // Verify every interval gap and both boundaries against the independent
        // registry/name evidence, not merely a repeated interval matcher.
        for raw in 0..=block_metadata("firefly_bush")
            .expect("final pinned block")
            .max_state
        {
            let expected = state_id_to_block_name(raw).and_then(|name| {
                EXPECTED_NAMES
                    .iter()
                    .find(|entry| entry.0 == name)
                    .map(|entry| entry.1)
            });
            assert_eq!(
                default_render_type(BlockStateId::new(raw)),
                expected,
                "raw {raw}"
            );
        }
        assert_eq!(default_render_type(BlockStateId::new(u32::MAX)), None);
    }

    #[test]
    fn all84_default_blocks_emit_correct_fresh_client_descriptor_without_world_mutation() {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0));
        for (index, &(name, _)) in EXPECTED_NAMES.iter().enumerate() {
            place(
                &mut chunk,
                BlockPos::new(
                    i32::try_from(index % 16).unwrap(),
                    -64,
                    i32::try_from(index / 16).unwrap(),
                ),
                name,
            );
        }
        let before = chunk.clone();
        let mut entries = Vec::new();
        append_state_render_entities(&chunk, &mut entries).expect("bounded descriptors");
        assert_eq!(entries.len(), 84);
        for (index, entry) in entries.iter().enumerate() {
            let &(name, kind) = EXPECTED_NAMES.get(index).expect("independent entry");
            assert_eq!(
                entry.packed_xz(),
                u8::try_from(((index % 16) << 4) | (index / 16)).unwrap(),
                "{name}"
            );
            assert_eq!(entry.y(), -64);
            assert_eq!(entry.block_entity_type(), kind, "{name}");
            assert_eq!(entry.data(), &NbtTag::Compound(NbtCompound::new()));
        }
        assert_eq!(chunk, before);
        assert_eq!(chunk.block_entity_count(), 0);
    }

    #[test]
    fn negative_columns_preserve_local_coordinates_and_both_signed_y_extremes() {
        let mut chunk = Chunk::new(ChunkPos::new(-2, -3));
        place(&mut chunk, BlockPos::new(-17, -64, -33), "red_bed");
        place(&mut chunk, BlockPos::new(-32, 319, -48), "bell");
        let mut entries = Vec::new();
        append_state_render_entities(&chunk, &mut entries).unwrap();
        assert_eq!(
            (
                entries[0].packed_xz(),
                entries[0].y(),
                entries[0].block_entity_type()
            ),
            (255, -64, 25)
        );
        assert_eq!(
            (
                entries[1].packed_xz(),
                entries[1].y(),
                entries[1].block_entity_type()
            ),
            (0, 319, 31)
        );
    }

    #[test]
    fn both_real_bed_parts_emit_descriptors() {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0));
        for (z, part) in [(1, "foot"), (0, "head")] {
            let properties = BTreeMap::from([("part", part), ("facing", "north")]);
            let raw = compute_state_id("red_bed", &properties).expect("real part state");
            chunk
                .set_block(BlockPos::new(0, 64, z), BlockStateId::new(raw))
                .unwrap();
        }
        let mut entries = Vec::new();
        append_state_render_entities(&chunk, &mut entries).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|entry| entry.block_entity_type() == 25));
        assert_eq!(entries[0].packed_xz(), 0);
        assert_eq!(entries[1].packed_xz(), 1);
    }

    #[test]
    fn unrelated_unknown_and_existing_typed_or_sign_states_are_not_duplicated() {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0));
        for (x, name) in [
            "stone",
            "chest",
            "trapped_chest",
            "furnace",
            "oak_sign",
            "oak_hanging_sign",
            "spawner",
            "end_portal",
            "moving_piston",
            "campfire",
            "lectern",
            "barrel",
        ]
        .into_iter()
        .enumerate()
        {
            place(
                &mut chunk,
                BlockPos::new(i32::try_from(x).unwrap(), 64, 0),
                name,
            );
        }
        chunk
            .set_block(BlockPos::new(15, 64, 15), BlockStateId::new(u32::MAX))
            .unwrap();
        let existing = vec![descriptor(1), descriptor(2), descriptor(0)];
        let mut entries = existing.clone();
        append_state_render_entities(&chunk, &mut entries).unwrap();
        assert_eq!(entries, existing);
    }

    #[test]
    fn actual_current_state_controls_descriptor_even_with_stale_typed_data() {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0));
        let position = BlockPos::new(0, 64, 0);
        place(&mut chunk, position, "red_bed");
        chunk
            .set_block_entity(position, BlockEntity::Chest(ChestInventory::new()))
            .unwrap();
        let mut entries = Vec::new();
        append_state_render_entities(&chunk, &mut entries).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].block_entity_type(), 25);
        place(&mut chunk, position, "stone");
        let mut entries = Vec::new();
        append_state_render_entities(&chunk, &mut entries).unwrap();
        assert_eq!(entries, Vec::<ChunkBlockEntity>::new());
        assert!(matches!(
            chunk.block_entity(position),
            Some(BlockEntity::Chest(_))
        ));
    }

    fn dense(count: usize) -> Chunk {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0));
        for index in 0..count {
            place(
                &mut chunk,
                BlockPos::new(
                    i32::try_from(index % 16).unwrap(),
                    -64 + i32::try_from(index / 256).unwrap(),
                    i32::try_from((index / 16) % 16).unwrap(),
                ),
                "red_bed",
            );
        }
        chunk
    }

    #[test]
    fn exact_shared_cap_succeeds_and_late_overflow_preserves_all_existing_entries() {
        let mut entries = Vec::new();
        append_state_render_entities(&dense(MAX_BLOCK_ENTITIES), &mut entries).unwrap();
        assert_eq!(entries.len(), MAX_BLOCK_ENTITIES);
        let existing = vec![descriptor(1)];
        let mut entries = existing.clone();
        assert!(append_state_render_entities(&dense(MAX_BLOCK_ENTITIES), &mut entries).is_err());
        assert_eq!(entries, existing);
        let mut entries = Vec::new();
        assert!(
            append_state_render_entities(&dense(MAX_BLOCK_ENTITIES + 1), &mut entries).is_err()
        );
        assert_eq!(entries, Vec::<ChunkBlockEntity>::new());
        let mut entries = vec![descriptor(0); MAX_BLOCK_ENTITIES];
        append_state_render_entities(&Chunk::new(ChunkPos::new(0, 0)), &mut entries).unwrap();
        entries.push(descriptor(0));
        let existing = entries.clone();
        assert!(
            append_state_render_entities(&Chunk::new(ChunkPos::new(0, 0)), &mut entries).is_err()
        );
        assert_eq!(entries, existing);
    }

    #[test]
    fn real_codec_has_exact_six_byte_default_schema_and_rejects_truncation() {
        for kind in [3, 13, 16, 20, 24, 25, 26, 31, 41] {
            let entry = ChunkBlockEntity::new(255, -64, kind, NbtTag::Compound(NbtCompound::new()));
            let mut encoded = BytesMut::new();
            entry.encode(&mut encoded).unwrap();
            assert_eq!(
                &encoded[..],
                &[255, 255, 192, u8::try_from(kind).unwrap(), 10, 0]
            );
            let mut reader = BoundedReader::new(&encoded);
            assert_eq!(ChunkBlockEntity::decode(&mut reader).unwrap(), entry);
            assert_eq!(reader.remaining(), 0);
            for end in 0..encoded.len() {
                assert!(
                    ChunkBlockEntity::decode(&mut BoundedReader::new(&encoded[..end])).is_err()
                );
            }
        }
        assert_eq!(std::mem::size_of::<(u8, i16, i32)>(), 8);
    }
}
