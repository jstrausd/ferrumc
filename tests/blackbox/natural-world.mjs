#!/usr/bin/env node
// Independent protocol terrain, creative edit, exploration and graceful restart gate.
// This covers the declared natural preview, not full Survival, lighting or Paper parity.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawn } from 'node:child_process';
import mc from 'minecraft-protocol';
import Registry from 'prismarine-registry';
import ChunkLoader from 'prismarine-chunk';
import Vec3pkg from 'vec3';
const Vec3 = Vec3pkg.Vec3 || Vec3pkg;
const registry = Registry('1.21.8');
const Chunk = ChunkLoader(registry);
const markerItem = registry.itemsByName.gold_block;
const markerBlock = registry.blocksByName.gold_block;
assert(markerItem && markerBlock, 'independent 1.21.8 gold-block registry');
const local = p => new Vec3(((p.x % 16) + 16) % 16, p.y, ((p.z % 16) + 16) % 16);
const columnKey = p => `${Math.floor(p.x / 16)},${Math.floor(p.z / 16)}`;
const blockKey = p => `${p.x},${p.y},${p.z}`;
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function bounded(promise, ms, label) {
  let timer;
  try { return await Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`${label} timeout`)), ms); })]); }
  finally { clearTimeout(timer); }
}
function inspectColumn(chunk, x, z, excluded) {
  const digest = crypto.createHash('sha256'), terrain = crypto.createHash('sha256'), biomeHash = crypto.createHash('sha256');
  const heights = [], biomes = new Set(); let solid = 0, undergroundAir = 0;
  const data = Buffer.alloc(4);
  for (let lz = 0; lz < 16; lz++) for (let lx = 0; lx < 16; lx++) {
    let top;
    for (let y = 319; y >= -64; y--) {
      const state = chunk.getBlockStateId(new Vec3(lx, y, lz)); data.writeUInt32BE(state); digest.update(data);
      if (!excluded || excluded.x !== x * 16 + lx || excluded.y !== y || excluded.z !== z * 16 + lz) terrain.update(data);
      if (state !== 0) { solid++; if (top === undefined) top = y; } else if (y < 40 && y > -50) undergroundAir++;
    }
    assert(top !== undefined, 'natural column has material'); heights.push(top);
  }
  // Hash every stored quart biome through the independent section decoder.
  for (let y = -64; y < 320; y += 4) for (let lz = 0; lz < 16; lz += 4) for (let lx = 0; lx < 16; lx += 4) {
    const biome = chunk.getBiome(new Vec3(lx, y, lz)); assert(Number.isInteger(biome), 'decoded quart biome id');
    data.writeUInt32BE(biome); biomeHash.update(data); biomes.add(biome);
  }
  assert(solid > 256, 'real terrain blocks'); assert(heights.some(y => y !== 63), 'not flat-v1');
  return { chunk: [x, z], hash: digest.digest('hex'), terrainHash: terrain.digest('hex'), biomeHash: biomeHash.digest('hex'), heights: [Math.min(...heights), Math.max(...heights)], biomes: [...biomes].sort((a, b) => a - b), solid, undergroundAir };
}
if (process.argv.includes('--self-test')) {
  const chunk = new Chunk({ x: 0, z: 0 });
  for (let z = 0; z < 16; z++) for (let x = 0; x < 16; x++) for (let y = -64; y < -62; y++) chunk.setBlockStateId(new Vec3(x, y, z), registry.blocksByName.stone.defaultState);
  const marker = { x: 10, y: -62, z: 8 };
  const before = inspectColumn(chunk, 0, 0, marker);
  chunk.setBlockStateId(local(marker), markerBlock.defaultState);
  const after = inspectColumn(chunk, 0, 0, marker);
  assert.notEqual(before.hash, after.hash); assert.equal(before.terrainHash, after.terrainHash); assert.equal(before.biomeHash, after.biomeHash);
  assert.equal(chunk.getBlock(local(marker)).name, 'gold_block');
  console.log('PASS independent marker and untouched-terrain hash self-test');
} else {
  const bin = process.env.MC_SERVER_BIN && path.resolve(process.env.MC_SERVER_BIN);
  assert(bin && fs.statSync(bin).isFile(), 'MC_SERVER_BIN must point to a prebuilt server');
  const binaryHash = crypto.createHash('sha256').update(fs.readFileSync(bin)).digest('hex');
  const dir = process.env.MC_RUN_DIR ? path.resolve(process.env.MC_RUN_DIR) : fs.mkdtempSync(path.join(os.tmpdir(), 'ferrumc-natural-world-'));
  const port = Number(process.env.MC_PORT || 25567);
  assert(Number.isInteger(port) && port > 0 && port <= 65535, 'MC_PORT must be 1..65535');
  const exploreEnabled = process.env.MC_NATURAL_EXPLORE !== '0';
  fs.mkdirSync(dir, { recursive: true });
  assert(!fs.existsSync(path.join(dir, 'world')), 'use a fresh MC_RUN_DIR; artifacts are preserved for diagnosis');
  const config = path.join(dir, 'config.toml');
  fs.writeFileSync(config, [
    `bind = ${JSON.stringify(`127.0.0.1:${port}`)}`, `world_dir = ${JSON.stringify(path.join(dir, 'world'))}`,
    'view_distance = 1', 'simulation_distance = 1', 'spawn_chunk_radius = 0', 'spawn = [8.0, 64.0, 8.0]',
    'dashboard_enabled = false', 'builtin_plugins = false', 'ops = ["NaturalProbe"]', '[world]', 'seed = 0', 'preset = "ferrumc_natural_preview"', '',
  ].join('\n'));
  console.log(`Natural preview artifacts: ${dir}\nBinary SHA-256: ${binaryHash}`);
  let child, exited, serverError;
  const connections = new Set();
  async function until(predicate, label, connection, ms = 180000) {
    const deadline = Date.now() + ms;
    while (true) {
      if (serverError) throw serverError;
      if (!child || child.exitCode !== null || child.signalCode !== null) throw new Error(`server exited during ${label}`);
      if (connection?.state.error) throw connection.state.error;
      if (connection?.state.ended) throw new Error(`client ended during ${label}: ${connection.state.reason}`);
      if (predicate()) return;
      if (Date.now() >= deadline) throw new Error(`${label} timeout`);
      await delay(25);
    }
  }
  async function start(round) {
    assert.equal(crypto.createHash('sha256').update(fs.readFileSync(bin)).digest('hex'), binaryHash, 'same executable across restart');
    const output = fs.openSync(path.join(dir, `server-${round}.log`), 'w');
    serverError = null;
    try { child = spawn(bin, ['--config', config], { cwd: dir, stdio: ['ignore', output, output] }); }
    finally { fs.closeSync(output); }
    exited = new Promise(resolve => {
      child.once('error', error => { serverError = error; resolve({ error }); });
      child.once('exit', (code, signal) => resolve({ code, signal }));
    });
    const deadline = Date.now() + 300000;
    while (Date.now() < deadline) {
      if (serverError) throw serverError;
      assert(child.exitCode === null && child.signalCode === null, `server exited: ${child.exitCode}/${child.signalCode}`);
      try {
        const status = await bounded(mc.ping({ host: '127.0.0.1', port, version: '1.21.8', closeTimeout: 2000 }), 3000, 'status ping');
        assert.equal(status.version.protocol, 772, 'independent protocol 772 status'); return;
      } catch { await delay(500); }
    }
    throw new Error('natural startup timeout');
  }
  async function stop() {
    if (!child) return;
    const stopping = child;
    try {
      assert(stopping.exitCode === null && stopping.signalCode === null, 'server running before graceful save/stop');
      stopping.kill('SIGINT');
      assert.deepEqual(await bounded(exited, 30000, 'graceful shutdown'), { code: 0, signal: null }, 'clean save/stop');
    } catch (error) {
      if (stopping.exitCode === null && stopping.signalCode === null) stopping.kill('SIGKILL');
      await bounded(exited, 10000, 'forced cleanup'); throw error;
    } finally { child = null; }
  }
  async function connect() {
    const client = mc.createClient({ host: '127.0.0.1', port, username: 'NaturalProbe', version: '1.21.8', auth: 'offline', keepAlive: false });
    const state = { columns: new Map(), changes: new Map(), acks: new Set(), slots: new Map(), positionRevision: 0, slotRevision: 0, expectedEnd: false };
    const connection = { client, state }; connections.add(connection);
    connection.ended = new Promise(resolve => client.once('end', reason => { state.ended = true; state.reason = reason; resolve(); }));
    client.on('error', error => { if (!state.expectedEnd) state.error = error; });
    for (const packet of ['kick_disconnect', 'disconnect']) client.on(packet, p => { if (!state.expectedEnd) state.error = new Error(JSON.stringify(p)); });
    client.on('keep_alive', p => client.write('keep_alive', { keepAliveId: p.keepAliveId }));
    client.on('login', p => { state.mode = p.worldState?.gamemode; });
    client.on('position', p => { state.position = { x: p.x, y: p.y, z: p.z }; state.positionRevision++; client.write('teleport_confirm', { teleportId: p.teleportId }); });
    client.on('map_chunk', p => {
      try { const chunk = new Chunk({ x: p.x, z: p.z }); chunk.load(p.chunkData, true, false); state.columns.set(`${p.x},${p.z}`, chunk); }
      catch (error) { state.error = error; }
    });
    client.on('unload_chunk', p => state.columns.delete(`${p.chunkX},${p.chunkZ}`));
    client.on('block_change', p => {
      state.changes.set(blockKey(p.location), p.type);
      state.columns.get(columnKey(p.location))?.setBlockStateId(local(p.location), p.type);
    });
    client.on('acknowledge_player_digging', p => state.acks.add(p.sequenceId));
    client.on('window_items', p => { if (p.windowId === 0) for (const [slot, item] of p.items.entries()) state.slots.set(slot, { item, revision: ++state.slotRevision }); });
    client.on('set_slot', p => { if (p.windowId === 0) state.slots.set(p.slot, { item: p.item, revision: ++state.slotRevision }); });
    await until(() => state.position && state.columns.has('0,0') && state.slots.has(36), 'spawn chunk, teleport and inventory', connection);
    assert.equal(state.mode, 'creative', 'genuine creative-mode fixture');
    return connection;
  }
  async function close(connection) {
    connection.state.expectedEnd = true; connection.client.end();
    try { await bounded(connection.ended, 10000, 'client disconnect'); }
    finally { connection.client.socket?.destroy(); connections.delete(connection); }
  }
  async function teleport(connection, position) {
    const before = connection.state.positionRevision;
    connection.client.write('chat_command', { command: `tp ${position.x} ${position.y} ${position.z}` });
    await until(() => connection.state.positionRevision > before, 'operator exploration teleport', connection, 30000);
    assert.deepEqual(connection.state.position, position, 'server-confirmed exploration position');
  }
  async function explore(connection, spawnPosition) {
    if (!exploreEnabled) return { status: 'SKIPPED_MC_NATURAL_EXPLORE_0' };
    const evidence = [];
    for (const x of [24, -8]) {
      const position = { x, y: spawnPosition.y + 10, z: 8 }, cx = Math.floor(x / 16);
      await teleport(connection, position);
      await until(() => connection.state.columns.has(`${cx},0`), `exploration chunk ${cx},0`, connection);
      evidence.push(inspectColumn(connection.state.columns.get(`${cx},0`), cx, 0));
    }
    await teleport(connection, spawnPosition);
    await until(() => connection.state.columns.has('0,0'), 'return to edited spawn chunk', connection);
    return { status: 'POSITIVE_AND_NEGATIVE_CHUNKS', columns: evidence };
  }
  function chooseMarker(connection) {
    const spawn = connection.state.position, chunk = connection.state.columns.get('0,0');
    for (const [dx, dz] of [[2, 0], [-2, 0], [0, 2], [0, -2]]) {
      const x = Math.floor(spawn.x) + dx, z = Math.floor(spawn.z) + dz;
      if (x < 0 || x >= 16 || z < 0 || z >= 16) continue;
      for (let y = Math.min(318, Math.floor(spawn.y) + 2); y >= Math.max(-64, Math.floor(spawn.y) - 3); y--) {
        const support = chunk.getBlock(new Vec3(x, y, z));
        const target = { x, y: y + 1, z };
        if (support.boundingBox === 'block' && chunk.getBlock(local(target)).name === 'air' &&
            (x + 0.5 - spawn.x) ** 2 + (target.y + 0.5 - (spawn.y + 1.62)) ** 2 + (z + 0.5 - spawn.z) ** 2 <= 25) return target;
      }
    }
    throw new Error('no dry surface marker cell within creative reach near safe spawn');
  }
  async function placeMarker(connection, marker) {
    const { client, state } = connection;
    const before = state.slotRevision;
    client.write('held_item_slot', { slotId: 0 });
    client.write('set_creative_slot', { slot: 36, item: { itemCount: 1, itemId: markerItem.id, addedComponentCount: 0, removedComponentCount: 0, components: [], removeComponents: [] } });
    await until(() => state.slots.get(36)?.revision > before, 'creative gold-block slot echo', connection, 30000);
    assert.equal(state.slots.get(36).item.itemId, markerItem.id, 'authoritative creative item');
    const sequence = 1;
    client.write('block_place', { hand: 0, location: { ...marker, y: marker.y - 1 }, direction: 1, cursorX: 0.5, cursorY: 1, cursorZ: 0.5, insideBlock: false, worldBorderHit: false, sequence });
    await until(() => state.changes.has(blockKey(marker)) && state.acks.has(sequence), 'gold-block update and action ACK', connection, 30000);
    assert.equal(state.changes.get(blockKey(marker)), markerBlock.defaultState, 'independent gold-block state id');
    assert.equal(state.columns.get('0,0').getBlock(local(marker)).name, 'gold_block', 'actual creative edit applied');
    return { position: marker, stateId: markerBlock.defaultState, name: 'gold_block', sequence };
  }
  try {
    await start(1);
    const firstClient = await connect(), spawnPosition = { ...firstClient.state.position }, marker = chooseMarker(firstClient);
    const before = inspectColumn(firstClient.state.columns.get('0,0'), 0, 0, marker);
    const edit = await placeMarker(firstClient, marker);
    const edited = inspectColumn(firstClient.state.columns.get('0,0'), 0, 0, marker);
    assert.notEqual(before.hash, edited.hash, 'creative edit changes actual terrain');
    assert.equal(before.terrainHash, edited.terrainHash, 'edit changes exactly its marker cell');
    const firstExploration = await explore(firstClient, spawnPosition);
    await close(firstClient); await stop();
    await start(2);
    const secondClient = await connect();
    const restarted = inspectColumn(secondClient.state.columns.get('0,0'), 0, 0, marker);
    assert.equal(secondClient.state.columns.get('0,0').getBlock(local(marker)).name, 'gold_block', 'saved creative edit streams after restart');
    assert.equal(edited.hash, restarted.hash, 'all edited natural blocks survive restart');
    assert.equal(before.terrainHash, restarted.terrainHash, 'untouched natural terrain survives restart');
    assert.equal(before.biomeHash, restarted.biomeHash, 'every stored biome survives restart');
    assert.deepEqual(secondClient.state.position, spawnPosition, 'returned safe spawn position survives restart');
    const secondExploration = await explore(secondClient, spawnPosition);
    assert.deepEqual(firstExploration, secondExploration, 'positive and negative natural exploration regenerates identically');
    await close(secondClient); await stop();
    const evidence = { status: 'PASS_NATURAL_PROTOCOL_EDIT_RESTART', scope: 'natural preview terrain and stored biomes, creative edit, graceful restart; no full Survival/Paper acceptance', binaryHash, spawn: spawnPosition, edit, before, edited, restarted, firstExploration, secondExploration };
    fs.writeFileSync(path.join(dir, 'acceptance.json'), JSON.stringify(evidence, null, 2));
    console.log(JSON.stringify(evidence));
  } finally {
    for (const connection of connections) { connection.state.expectedEnd = true; connection.client.socket?.destroy(); }
    if (child) await stop();
  }
}
