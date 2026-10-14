//! Bounded asynchronous relighting of immutable resident chunk snapshots.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ferrumc_codec::BoundedBytes;
use ferrumc_math::ChunkPos;
use ferrumc_proto::generated::play::{ClientboundPlayPacket, UpdateLight};
use ferrumc_sim::LoadedChunkMap;
use ferrumc_world::{ChunkLightData, LightingNeighborhood};
use tokio::task::JoinSet;

const JOBS: usize = 2;
const TARGETS: usize = 256;
// Config allows 1.1M view slots; one retained ring adds at most264 per1024
// viewers, and the configured spawn set adds289. Pending keys own no snapshots.
const MAX_PENDING_CENTRES: usize = 1_100_000 + 1024 * 264 + 289;

struct Version {
    epoch: u64,
    queued: bool,
    running: bool,
}

/// Two admitted CPU jobs, 256 active centre versions and a config-bounded set of deferred keys.
///
/// Jobs own at most nine immutable chunk copies each. Producer scratch, snapshots
/// and result arrays are additional to the generation preparation cache.
pub(crate) struct Relighter {
    versions: BTreeMap<ChunkPos, Version>,
    queue: VecDeque<ChunkPos>,
    pending: BTreeSet<ChunkPos>,
    cached: BTreeMap<ChunkPos, ClientboundPlayPacket>,
    exhausted: bool,
    jobs: JoinSet<(ChunkPos, u64, Result<ChunkLightData, String>)>,
}

impl Relighter {
    pub(crate) fn new() -> Self {
        Self {
            versions: BTreeMap::new(),
            queue: VecDeque::new(),
            pending: BTreeSet::new(),
            cached: BTreeMap::new(),
            exhausted: false,
            jobs: JoinSet::new(),
        }
    }

    /// Reads only current computed light; dirty/missing columns refuse candidates.
    pub(crate) fn resident_light(
        &self,
        position: ferrumc_math::BlockPos,
    ) -> Option<crate::light_view::ResidentLight> {
        let ClientboundPlayPacket::UpdateLight(packet) =
            self.cached.get(&position.to_chunk_pos())?
        else {
            return None;
        };
        crate::light_view::read_light(position, packet)
    }

    /// At most64 immutable computed column keys; no snapshots or light copies.
    pub(crate) fn resident_columns(&self) -> Vec<ChunkPos> {
        self.cached.keys().copied().collect()
    }

    /// Requests this centre after its block or loaded-neighbour input changed.
    pub(crate) fn request(&mut self, pos: ChunkPos) {
        self.cached.remove(&pos);
        if !self.versions.contains_key(&pos) && self.versions.len() >= TARGETS {
            if !self.pending.contains(&pos) && self.pending.len() >= MAX_PENDING_CENTRES {
                self.exhausted = true;
            } else {
                self.pending.insert(pos);
            }
            return;
        }
        let version = self.versions.entry(pos).or_insert(Version {
            epoch: 0,
            queued: false,
            running: false,
        });
        self.cached.remove(&pos);
        let Some(epoch) = version.epoch.checked_add(1) else {
            self.exhausted = true;
            return;
        };
        version.epoch = epoch;
        if !version.running && !version.queued {
            version.queued = true;
            self.queue.push_back(pos);
        }
    }

    pub(crate) fn ensure(&mut self, pos: ChunkPos) -> Option<ClientboundPlayPacket> {
        if let Some(packet) = self.cached.get(&pos) {
            return Some(packet.clone());
        }
        if !self.versions.contains_key(&pos) && !self.pending.contains(&pos) {
            self.request(pos);
        }
        None
    }

    pub(crate) fn changed(&mut self, pos: ChunkPos, chunks: &LoadedChunkMap) {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let Some(x) = pos.x().checked_add(dx) else {
                    continue;
                };
                let Some(z) = pos.z().checked_add(dz) else {
                    continue;
                };
                let neighbour = ChunkPos::new(x, z);
                if chunks.get(neighbour).is_some() {
                    self.request(neighbour);
                }
            }
        }
    }

    /// Publishes only results whose exact snapshot epoch remains current.
    pub(crate) fn pump(
        &mut self,
        chunks: &LoadedChunkMap,
    ) -> Result<Vec<(ChunkPos, ClientboundPlayPacket)>, String> {
        if self.exhausted {
            return Err("lighting snapshot epoch exhausted".to_owned());
        }
        self.cached.retain(|pos, _| chunks.get(*pos).is_some());
        self.pending.retain(|pos| chunks.get(*pos).is_some());
        let mut ready = Vec::new();
        while let Some(result) = self.jobs.try_join_next() {
            let (pos, epoch, light) = result.map_err(|e| e.to_string())?;
            let Some(version) = self.versions.get_mut(&pos) else {
                continue;
            };
            version.running = false;
            if chunks.get(pos).is_none() {
                self.versions.remove(&pos);
                continue;
            }
            if version.epoch != epoch {
                if !version.queued {
                    version.queued = true;
                    self.queue.push_back(pos);
                }
                continue;
            }
            let light = light?;
            let packet = packet(pos, &light)?;
            if self.cached.len() >= 64 {
                self.cached.pop_first();
            }
            self.cached.insert(pos, packet.clone());
            ready.push((pos, packet));
            self.versions.remove(&pos);
        }
        // Promote deferred dirtiness as space becomes available. Coalescing needs
        // no stale snapshot: the eventual worker reads the then-current resident data.
        while self.versions.len() < TARGETS {
            let Some(pos) = self.pending.pop_first() else {
                break;
            };
            if chunks.get(pos).is_some() {
                self.request(pos);
            }
        }
        while self.jobs.len() < JOBS {
            let Some(pos) = self.queue.pop_front() else {
                break;
            };
            let Some(version) = self.versions.get_mut(&pos) else {
                continue;
            };
            version.queued = false;
            if chunks.get(pos).is_none() {
                self.versions.remove(&pos);
                continue;
            }
            let epoch = version.epoch;
            version.running = true;
            let mut snapshots = Vec::with_capacity(9);
            for dx in -1..=1 {
                for dz in -1..=1 {
                    let Some(x) = pos.x().checked_add(dx) else {
                        continue;
                    };
                    let Some(z) = pos.z().checked_add(dz) else {
                        continue;
                    };
                    if let Some(chunk) = chunks.get(ChunkPos::new(x, z)) {
                        snapshots.push(chunk.clone());
                    }
                }
            }
            self.jobs.spawn_blocking(move || {
                let references: Vec<_> = snapshots.iter().collect();
                let output = LightingNeighborhood::new(pos, &references)
                    .and_then(|input| input.compute())
                    .map_err(|e| e.to_string());
                (pos, epoch, output)
            });
        }
        Ok(ready)
    }

    /// Stop waits admitted work without admitting queued requests.
    pub(crate) async fn drain(&mut self) -> Result<(), String> {
        while let Some(result) = self.jobs.join_next().await {
            let (_, _, result) = result.map_err(|e| e.to_string())?;
            result?;
        }
        self.versions.clear();
        self.cached.clear();
        self.queue.clear();
        self.pending.clear();
        Ok(())
    }
}

fn packet(pos: ChunkPos, light: &ChunkLightData) -> Result<ClientboundPlayPacket, String> {
    let bounded = |arrays: &[Vec<u8>]| -> Result<Vec<BoundedBytes<2048>>, String> {
        arrays
            .iter()
            .map(|a| BoundedBytes::new(a.clone()).map_err(|e| e.to_string()))
            .collect()
    };
    Ok(ClientboundPlayPacket::UpdateLight(UpdateLight::new(
        pos.x(),
        pos.z(),
        light.sky_light_mask().to_vec(),
        light.block_light_mask().to_vec(),
        light.empty_sky_light_mask().to_vec(),
        light.empty_block_light_mask().to_vec(),
        bounded(light.sky_light())?,
        bounded(light.block_light())?,
    )))
}
