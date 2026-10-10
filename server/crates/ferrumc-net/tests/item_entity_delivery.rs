//! Transport acceptance for NET-04.1b using real writer queues and frame bytes.
//! These tests do not activate a viewer, validate metadata semantics, or prove
//! the caller's mandatory-refusal disconnect or atomic bundle admission.

use ferrumc_codec::{BoundedBytes, BoundedReader};
use ferrumc_net::{
    decode_inbound_frame, BatchLimits, CompressionState, ConnectionLimits, ConnectionState,
    Criticality, DecodeOutcome, EncodeError, EnqueueOutcome, FrameEncodeError, InboundPacket,
    OutboundPriority, PlayBatch, PlayWriter,
};
use ferrumc_proto::generated::play::{
    ClientboundKeepAlive, ClientboundPlayPacket, EntityVelocity, RemoveEntities, SetEntityData,
    SetEntityMotion, SpawnEntity, TakeItemEntity, UnloadChunk, UpdateTime,
};

const ENTITY_ID: i32 = 255;
const METADATA_CAP: usize = 263_168;
// Actual metadata tails exported through original Paper Build60 STREAM_CODEC,
// independently reproduced by Prismarine in NET-04.1's pinned fixture.
const STONE_1: &[u8] = &[0x08, 0x07, 0x01, 0x01, 0x00, 0x00, 0xff];
const STONE_64: &[u8] = &[0x08, 0x07, 0x40, 0x01, 0x00, 0x00, 0xff];

fn spawn() -> ClientboundPlayPacket {
    // minecraft-data 3.111.0, Minecraft 1.21.8/protocol772: item entity type69.
    ClientboundPlayPacket::SpawnEntity(SpawnEntity::new(
        ENTITY_ID,
        uuid::Uuid::nil(),
        69,
        -0.5,
        64.25,
        16.0,
        0,
        0,
        0,
        0,
        EntityVelocity::new(0, 0, 0),
    ))
}

fn metadata(raw: &[u8]) -> ClientboundPlayPacket {
    ClientboundPlayPacket::SetEntityData(SetEntityData::new(
        ENTITY_ID,
        BoundedBytes::<METADATA_CAP>::new(raw.to_vec()).expect("bounded fixture tail"),
    ))
}

fn motion() -> ClientboundPlayPacket {
    ClientboundPlayPacket::SetEntityMotion(SetEntityMotion::new(ENTITY_ID, i16::MIN, 0, i16::MAX))
}

fn take(count: i32) -> ClientboundPlayPacket {
    ClientboundPlayPacket::TakeItemEntity(TakeItemEntity::new(ENTITY_ID, 128, count))
}

fn remove() -> ClientboundPlayPacket {
    ClientboundPlayPacket::RemoveEntities(RemoveEntities::new(vec![ENTITY_ID]))
}

fn writer(state_capacity: usize, batch: BatchLimits) -> PlayWriter {
    PlayWriter::new(
        ConnectionLimits::default(),
        PlayWriter::capacities_from(4, state_capacity, 4, 4),
        batch,
    )
}

fn decode_batch(batch: &PlayBatch, compression: &CompressionState) -> Vec<ClientboundPlayPacket> {
    let mut input = batch.bytes();
    let mut packets = Vec::new();
    while !input.is_empty() {
        let frame =
            decode_inbound_frame(input, ConnectionState::Play, &ConnectionLimits::default())
                .expect("complete writer frame");
        let DecodeOutcome::Decoded {
            packet: InboundPacket::Play(body),
            consumed,
        } = frame
        else {
            panic!("writer must produce a complete raw Play frame");
        };
        assert!(consumed > 0 && consumed <= input.len());
        let body = compression.decompress(&body).expect("writer compression");
        let mut reader = BoundedReader::new(&body);
        let id = reader.read_var_int().expect("packet id");
        let packet = ClientboundPlayPacket::decode(id, &mut reader).expect("clientbound body");
        assert_eq!(reader.remaining(), 0, "exact packet body consumed");
        packets.push(packet);
        input = &input[consumed..];
    }
    assert_eq!(packets.len(), batch.frame_count());
    packets
}

fn drain_all(
    writer: &mut PlayWriter,
    compression: &CompressionState,
) -> Vec<ClientboundPlayPacket> {
    let mut packets = Vec::new();
    let mut previous_depth = writer.total_queued();
    while previous_depth > 0 {
        let batch = writer.drain_batch(compression).expect("admitted frames");
        assert!(!batch.is_empty(), "nonempty queue must make progress");
        packets.extend(decode_batch(&batch, compression));
        let next_depth = writer.total_queued();
        assert_eq!(previous_depth - next_depth, batch.frame_count());
        previous_depth = next_depth;
    }
    let empty = writer.drain_batch(compression).expect("empty drain");
    assert!(empty.is_empty());
    assert_eq!(empty.bytes(), []);
    packets
}

#[test]
fn initial_state_fifo_preserves_spawn_metadata_motion_and_stop() {
    let packets = vec![
        spawn(),
        metadata(STONE_64),
        motion(),
        ClientboundPlayPacket::SetEntityMotion(SetEntityMotion::new(ENTITY_ID, 0, 0, 0)),
    ];
    let mut writer = writer(8, BatchLimits::default());
    for (index, packet) in packets.iter().enumerate() {
        assert_eq!(Criticality::for_packet(packet), Criticality::Mandatory);
        assert_eq!(
            writer.enqueue_classified(packet.clone()),
            EnqueueOutcome::Enqueued { depth: index + 1 }
        );
    }
    assert_eq!(writer.queued_len(OutboundPriority::State), packets.len());
    assert_eq!(writer.queued_len(OutboundPriority::World), 0);
    assert_eq!(
        drain_all(&mut writer, &CompressionState::disabled()),
        packets
    );
    assert_eq!(writer.metrics().frames_encoded(), 4);
    assert_eq!(writer.metrics().dropped_total(), 0);
}

#[test]
fn partial_metadata_and_original_take_counts_reach_frames_unchanged() {
    // A producer can animate the original64 although only part was moved.
    // Net transports these inputs; it neither computes pickup nor subtracts it.
    let packets = vec![
        metadata(STONE_64),
        take(64),
        metadata(STONE_1),
        take(1),
        remove(),
    ];
    let mut writer = writer(8, BatchLimits::new(1, 1));
    for packet in &packets {
        assert_eq!(Criticality::for_packet(packet), Criticality::Mandatory);
        assert!(writer.enqueue_classified(packet.clone()).is_enqueued());
    }
    assert_eq!(
        drain_all(&mut writer, &CompressionState::disabled()),
        packets
    );
    assert_eq!(writer.metrics().batches_flushed(), 5);
}

#[test]
fn mixed_priorities_keep_state_fifo_across_batch_splits_and_compression() {
    let state = [spawn(), metadata(STONE_1), motion(), take(64), remove()];
    let critical = ClientboundPlayPacket::ClientboundKeepAlive(ClientboundKeepAlive::new(123));
    let world = ClientboundPlayPacket::UnloadChunk(UnloadChunk::new(-1, 2));
    let cosmetic = ClientboundPlayPacket::UpdateTime(UpdateTime::new(100, 100, true));
    let mut expected = vec![critical.clone()];
    expected.extend_from_slice(&state);
    expected.extend([world.clone(), cosmetic.clone()]);

    for batch in [
        BatchLimits::default(),
        BatchLimits::new(64 * 1024, 1),
        BatchLimits::new(1, 128),
    ] {
        for compression in [
            CompressionState::disabled(),
            CompressionState::enabled(0),
            CompressionState::enabled(256),
        ] {
            let mut writer = writer(8, batch);
            for packet in [cosmetic.clone(), world.clone()] {
                assert!(writer.enqueue_classified(packet).is_enqueued());
            }
            for packet in &state {
                assert!(writer.enqueue_classified(packet.clone()).is_enqueued());
            }
            assert!(writer.enqueue_classified(critical.clone()).is_enqueued());
            assert_eq!(drain_all(&mut writer, &compression), expected);
            assert_eq!(writer.metrics().frames_encoded(), 8);
            assert_eq!(writer.metrics().dropped_total(), 0);
            if batch.max_frames() == 1 || batch.max_bytes() == 1 {
                assert_eq!(writer.metrics().batches_flushed(), 8);
            }
        }
    }
}

#[test]
fn maximum_admitted_raw_tail_makes_progress_under_soft_byte_cap() {
    // The cap admits opaque bytes, not a promise of valid item metadata.
    let packet = metadata(&vec![0x5a; METADATA_CAP]);
    let follower = motion();
    let mut writer = writer(2, BatchLimits::new(1, 128));
    assert!(writer.enqueue_classified(packet.clone()).is_enqueued());
    assert!(writer.enqueue_classified(follower.clone()).is_enqueued());
    let disabled = CompressionState::disabled();
    let batch = writer
        .drain_batch(&disabled)
        .expect("maximum admitted frame");
    assert_eq!(batch.frame_count(), 1);
    // Three-byte length, one-byte id, two-byte entity id, opaque tail.
    assert_eq!(batch.bytes().len(), METADATA_CAP + 6);
    assert_eq!(decode_batch(&batch, &disabled), vec![packet]);
    assert_eq!(writer.queued_len(OutboundPriority::State), 1);
    assert_eq!(drain_all(&mut writer, &disabled), vec![follower]);
    assert_eq!(writer.metrics().frames_encoded(), 2);
    assert_eq!(writer.metrics().dropped_total(), 0);
}

#[test]
fn full_state_queue_refuses_each_mandatory_follower_and_retains_fifo() {
    for refused in [metadata(STONE_64), motion(), take(64)] {
        let retained = vec![spawn(), metadata(STONE_1)];
        let mut writer = writer(2, BatchLimits::new(1, 1));
        assert_eq!(writer.capacity(OutboundPriority::State), 2);
        for (index, packet) in retained.iter().enumerate() {
            assert_eq!(
                writer.enqueue_classified(packet.clone()),
                EnqueueOutcome::Enqueued { depth: index + 1 }
            );
        }
        assert_eq!(Criticality::for_packet(&refused), Criticality::Mandatory);
        assert_eq!(
            writer.enqueue_classified(refused),
            EnqueueOutcome::Dropped {
                priority: OutboundPriority::State
            }
        );
        assert_eq!(writer.queued_len(OutboundPriority::State), 2);
        assert_eq!(writer.total_queued(), 2);
        assert_eq!(writer.metrics().dropped(OutboundPriority::State), 1);
        assert_eq!(writer.metrics().dropped_total(), 1);
        assert_eq!(writer.metrics().frames_encoded(), 0);
        assert_eq!(
            drain_all(&mut writer, &CompressionState::disabled()),
            retained
        );
        assert_eq!(writer.metrics().frames_encoded(), 2);
        // Dropped is the real Layer B refusal. The caller's Layer A criticality
        // makes it fatal; no writer-only assertion can prove that disconnect.
    }
}

#[test]
fn zero_state_capacity_refuses_all_new_packets_without_consuming_other_lanes() {
    let mut writer = writer(0, BatchLimits::default());
    let critical = ClientboundPlayPacket::ClientboundKeepAlive(ClientboundKeepAlive::new(9));
    assert!(writer.enqueue_classified(critical.clone()).is_enqueued());
    for refused in [metadata(STONE_1), motion(), take(64)] {
        assert_eq!(Criticality::for_packet(&refused), Criticality::Mandatory);
        assert_eq!(
            writer.enqueue_classified(refused),
            EnqueueOutcome::Dropped {
                priority: OutboundPriority::State
            }
        );
    }
    assert_eq!(writer.queued_len(OutboundPriority::State), 0);
    assert_eq!(writer.total_queued(), 1);
    assert_eq!(writer.metrics().dropped(OutboundPriority::State), 3);
    assert_eq!(writer.metrics().dropped_total(), 3);
    assert_eq!(
        drain_all(&mut writer, &CompressionState::disabled()),
        vec![critical]
    );
}

#[test]
fn droppable_world_refusal_has_its_own_outcome_without_displacing_state() {
    let mut writer = PlayWriter::new(
        ConnectionLimits::default(),
        PlayWriter::capacities_from(1, 4, 1, 1),
        BatchLimits::new(1, 1),
    );
    let state = [spawn(), metadata(STONE_1), motion()];
    let retained_world = ClientboundPlayPacket::UnloadChunk(UnloadChunk::new(1, 2));
    let refused_world = ClientboundPlayPacket::UnloadChunk(UnloadChunk::new(3, 4));
    assert!(writer
        .enqueue_classified(retained_world.clone())
        .is_enqueued());
    for packet in &state {
        assert!(writer.enqueue_classified(packet.clone()).is_enqueued());
    }
    assert_eq!(
        Criticality::for_packet(&refused_world),
        Criticality::Droppable
    );
    assert_eq!(
        writer.enqueue_classified(refused_world),
        EnqueueOutcome::Dropped {
            priority: OutboundPriority::World
        }
    );
    assert_eq!(writer.queued_len(OutboundPriority::World), 1);
    assert_eq!(writer.queued_len(OutboundPriority::State), 3);
    assert_eq!(writer.metrics().dropped(OutboundPriority::World), 1);
    assert_eq!(writer.metrics().dropped(OutboundPriority::State), 0);
    assert_eq!(writer.metrics().dropped_total(), 1);
    let mut expected = state.to_vec();
    expected.push(retained_world);
    assert_eq!(
        drain_all(&mut writer, &CompressionState::disabled()),
        expected
    );
}

#[test]
fn explicit_critical_metadata_override_overtakes_state_spawn() {
    let mut writer = writer(4, BatchLimits::new(1, 1));
    let spawn = spawn();
    let metadata = metadata(STONE_1);
    assert!(writer.enqueue_classified(spawn.clone()).is_enqueued());
    assert!(writer
        .enqueue(OutboundPriority::Critical, metadata.clone())
        .is_enqueued());
    assert_eq!(writer.queued_len(OutboundPriority::Critical), 1);
    assert_eq!(writer.queued_len(OutboundPriority::State), 1);
    assert_eq!(
        drain_all(&mut writer, &CompressionState::disabled()),
        vec![metadata, spawn]
    );
}

#[test]
fn explicit_world_pickup_override_is_overtaken_by_state_remove() {
    let mut writer = writer(4, BatchLimits::new(1, 1));
    let pickup = take(64);
    let remove = remove();
    assert!(writer
        .enqueue(OutboundPriority::World, pickup.clone())
        .is_enqueued());
    assert!(writer.enqueue_classified(remove.clone()).is_enqueued());
    assert_eq!(writer.queued_len(OutboundPriority::World), 1);
    assert_eq!(writer.queued_len(OutboundPriority::State), 1);
    assert_eq!(
        drain_all(&mut writer, &CompressionState::disabled()),
        vec![remove, pickup]
    );
}

#[test]
fn metadata_frame_cap_refusal_retains_front_followers_and_counters() {
    // Actual body: id5c + VarInt255(ff01) + seven fixture tail bytes =10.
    let packets = vec![metadata(STONE_1), motion(), remove()];
    let disabled = CompressionState::disabled();
    let mut writer = PlayWriter::new(
        ConnectionLimits::new(4096, 32768, 65536, 2 * 1024 * 1024, 9),
        PlayWriter::capacities_from(1, 4, 1, 1),
        BatchLimits::default(),
    );
    for packet in &packets {
        assert!(writer.enqueue_classified(packet.clone()).is_enqueued());
    }
    let error = writer
        .drain_batch(&disabled)
        .expect_err("ten-byte metadata exceeds nine");
    assert!(matches!(
        error,
        FrameEncodeError::Encode(EncodeError::FrameTooLarge {
            state: ConnectionState::Play,
            length: 10,
            max: 9,
        })
    ));
    assert_eq!(writer.total_queued(), 3);
    assert_eq!(writer.queued_len(OutboundPriority::State), 3);
    assert_eq!(writer.metrics().frames_encoded(), 0);
    assert_eq!(writer.metrics().batches_flushed(), 0);
    assert_eq!(writer.metrics().dropped_total(), 0);
    // A repeated caller attempt still sees the same offending front, not motion
    // or remove. Production callers close on this error instead of retrying.
    assert!(matches!(
        writer.drain_batch(&disabled),
        Err(FrameEncodeError::Encode(EncodeError::FrameTooLarge {
            length: 10,
            max: 9,
            ..
        }))
    ));
    assert_eq!(writer.total_queued(), 3);
    assert_eq!(writer.metrics().frames_encoded(), 0);
    assert_eq!(writer.metrics().batches_flushed(), 0);

    let mut exact = PlayWriter::new(
        ConnectionLimits::new(4096, 32768, 65536, 2 * 1024 * 1024, 10),
        PlayWriter::capacities_from(1, 4, 1, 1),
        BatchLimits::default(),
    );
    for packet in &packets {
        assert!(exact.enqueue_classified(packet.clone()).is_enqueued());
    }
    assert_eq!(drain_all(&mut exact, &disabled), packets);
    assert_eq!(exact.metrics().dropped_total(), 0);
}

#[test]
fn writer_transports_opaque_metadata_and_signed_take_without_reinterpretation() {
    let packets = vec![
        metadata(&[]),
        metadata(&[0xff, 0xa5]),
        metadata(&[0x08, 0x7f, 0x01]),
        take(i32::MIN),
        take(i32::MAX),
    ];
    let mut writer = writer(8, BatchLimits::default());
    for packet in &packets {
        assert!(writer.enqueue_classified(packet.clone()).is_enqueued());
    }
    assert_eq!(
        drain_all(&mut writer, &CompressionState::disabled()),
        packets
    );
}
