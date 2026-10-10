#!/usr/bin/env node
// BLK-01.1: independent item-placement, observer and process-restart acceptance.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import mc from 'minecraft-protocol';
import mcData from 'minecraft-data';
import mcRegistry from 'prismarine-registry';
import ChunkLoader from 'prismarine-chunk';
import BlockLoader from 'prismarine-block';
import Vec3pkg from 'vec3';

const version = '1.21.8';
const data = mcData(version);
const registry = mcRegistry(version);
const Chunk = ChunkLoader(registry);
const Block = BlockLoader(registry);
const Vec3 = Vec3pkg.Vec3 || Vec3pkg.default || Vec3pkg;
const host = '127.0.0.1';
const username = 'DoorHingeTester';
const binary = process.env.MC_SERVER_BIN ? path.resolve(process.env.MC_SERVER_BIN) : null;
const connections = new Set();
const expected = new Map();
let runDir;
let configPath;
let port;
let server;
let serverExit;
let serverError;
let serverLog;
let binaryHash;
let sequence = 0;

// These are explicit property goldens transcribed from the pinned original
// DoorBlock.getHinge, not computed by calling FerrumC's placement helpers.
const orientations = [
  { facing: 'south', yaw: 0, hinges: ['right', 'left', 'left'] },
  { facing: 'west', yaw: 90, hinges: ['right', 'left', 'left'] },
  { facing: 'north', yaw: 180, hinges: ['left', 'left', 'right'] },
  { facing: 'east', yaw: 270, hinges: ['left', 'left', 'right'] },
];
const cursors = [0.25, 0.5, 0.75];
const key = position => `${position.x},${position.y},${position.z}`;
const columnKey = position => `${Math.floor(position.x / 16)},${Math.floor(position.z / 16)}`;
const local = position => new Vec3(((position.x % 16) + 16) % 16, position.y, ((position.z % 16) + 16) % 16);
const decode = id => {
  const block = Block.fromStateId(id, 0);
  return { name: block.name, properties: block.getProperties() };
};

function goldenState(material, facing, hinge, half) {
  const properties = { facing, hinge, half, open: false, powered: false };
  const block = data.blocksByName[material];
  assert(block, `independent registry contains ${material}`);
  const matches = [];
  for (let id = block.minStateId; id <= block.maxStateId; id++) {
    if (JSON.stringify(Object.entries(decode(id).properties).sort()) === JSON.stringify(Object.entries(properties).sort())) matches.push(id);
  }
  assert.equal(matches.length, 1, `unique ${material} state for ${JSON.stringify(properties)}`);
  return { id: matches[0], name: material, properties };
}

function selfTest() {
  assert.equal(data.version.version, 772);
  const ids = new Set();
  for (const { facing } of orientations) for (const hinge of ['left', 'right']) for (const half of ['lower', 'upper']) {
    const golden = goldenState('oak_door', facing, hinge, half);
    assert.deepEqual(decode(golden.id), { name: golden.name, properties: golden.properties });
    ids.add(golden.id);
  }
  assert.equal(ids.size, 16);
  assert.throws(() => goldenState('oak_door', 'up', 'left', 'lower'), /unique oak_door state/);
  assert.throws(() => goldenState('missing_door', 'north', 'left', 'lower'), /independent registry/);
  console.log('PASS: independent registry identifies 16 unique closed/unpowered oak door states and rejects invalid goldens');
}

async function bounded(promise, label, timeout = 30_000) {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`Timeout: ${label}`)), timeout);
    })]);
  } finally { clearTimeout(timer); }
}

async function until(predicate, label, watched = [], timeout = 20_000) {
  const end = Date.now() + timeout;
  for (;;) {
    if (serverError) throw serverError;
    if (server && (server.exitCode !== null || server.signalCode !== null)) throw new Error(`Server exited during ${label}; inspect ${serverLog}`);
    for (const connection of watched) {
      if (connection.state.error) throw connection.state.error;
      if (connection.state.ended) throw new Error(`${connection.state.username} ended during ${label}: ${connection.state.reason}`);
    }
    if (predicate()) return;
    if (Date.now() >= end) throw new Error(`Timeout: ${label}`);
    // Time only bounds waiting for an observed packet/process condition. It
    // never proves an action completed or that an update was absent.
    await new Promise(resolve => setTimeout(resolve, 10));
  }
}

async function hashBinary() {
  const hash = createHash('sha256');
  for await (const bytes of fs.createReadStream(binary)) hash.update(bytes);
  return hash.digest('hex');
}

async function setup() {
  assert(binary && fs.statSync(binary).isFile(), 'MC_SERVER_BIN must point to a prebuilt frozen server binary');
  binaryHash = await bounded(hashBinary(), 'binary SHA-256');
  const requestedPort = Number(process.env.MC_PORT ?? 25717);
  assert(Number.isInteger(requestedPort) && requestedPort >= 0 && requestedPort <= 65535, 'MC_PORT must be 0..65535');
  const socket = net.createServer();
  const listenController = new AbortController();
  socket.on('connection', connection => connection.destroy());
  try {
    await bounded(new Promise((resolve, reject) => {
      socket.once('error', reject);
      socket.listen({ port: requestedPort, host, signal: listenController.signal }, resolve);
    }), 'reserve test port');
    port = socket.address().port;
  } finally {
    // Abort also cancels a listen that has not completed. Destroying incoming
    // probe connections prevents server.close from waiting on foreign sockets.
    listenController.abort();
    await bounded(new Promise((resolve, reject) => socket.close(error => {
      if (error && error.code !== 'ERR_SERVER_NOT_RUNNING') reject(error);
      else resolve();
    })), 'release test port');
  }
  runDir = fs.mkdtempSync(path.join(os.tmpdir(), 'ferrumc-door-hinge-'));
  configPath = path.join(runDir, 'config.toml');
  fs.writeFileSync(configPath, [
    `bind = ${JSON.stringify(`${host}:${port}`)}`,
    `world_dir = ${JSON.stringify(path.join(runDir, 'world'))}`,
    'dashboard_enabled = false', 'builtin_plugins = false',
    'spawn = [8.0, 64.0, 8.0]', 'spawn_chunk_radius = 1', 'view_distance = 2',
    `ops = [${JSON.stringify(username)}]`, 'keep_alive_interval_ms = 10000', '',
  ].join('\n'));
  console.log(`Binary: ${binary}\nSHA-256: ${binaryHash}\nArtifacts: ${runDir}\nPort: ${port}`);
}

async function start(label) {
  assert.equal(await bounded(hashBinary(), 'unchanged binary SHA-256'), binaryHash, 'both runs use the same frozen executable');
  serverLog = path.join(runDir, `server-${label}.log`);
  serverError = null;
  const out = fs.openSync(serverLog, 'w');
  try {
    server = spawn(binary, ['--config', configPath], {
      cwd: runDir, env: { ...process.env, RUST_LOG: 'info' }, stdio: ['ignore', out, out],
    });
    serverExit = new Promise(resolve => {
      server.once('error', error => { serverError = error; resolve({ error }); });
      server.once('exit', (code, signal) => resolve({ code, signal }));
    });
  } finally { fs.closeSync(out); }
  await until(() => fs.readFileSync(serverLog, 'utf8').includes('listening for Minecraft clients'), `${label} readiness`, [], 60_000);
}

async function stop() {
  if (!server) return;
  const child = server;
  const clients = [...connections];
  for (const connection of clients) connection.state.expectedEnd = true;
  let failure;
  try {
    assert(child.exitCode === null && child.signalCode === null, 'server must be running before SIGINT');
    child.kill('SIGINT');
    const result = await bounded(serverExit, 'graceful SIGINT shutdown');
    assert.deepEqual(result, { code: 0, signal: null });
    assert(fs.readFileSync(serverLog, 'utf8').includes('shutdown complete'), 'graceful save/drain completed');
    await bounded(Promise.all(clients.map(connection => connection.ended)), 'client sockets close after shutdown');
  } catch (error) {
    failure = error;
    if (child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
    try { await bounded(serverExit, 'forced process cleanup', 5000); }
    catch (cleanup) { console.error(`Process cleanup failed: ${cleanup.message}`); }
    for (const connection of clients) {
      connection.client.end('door acceptance cleanup');
      connection.client.socket?.destroy();
    }
  } finally { server = null; }
  if (failure) throw failure;
}

async function connect(name) {
  const state = {
    username: name, error: null, ended: false, expectedEnd: false, reason: '',
    joined: false, position: null, positionRevision: 0, teleports: new Set(),
    columns: new Map(), changes: new Map(), acks: new Set(),
    slots: new Map(), slotRevision: 0, mode: null,
  };
  const client = mc.createClient({ host, port, username: name, auth: 'offline', version, keepAlive: false });
  const connection = { client, state };
  connections.add(connection);
  connection.ended = new Promise(resolve => client.once('end', reason => {
    state.ended = true; state.reason ||= reason; connections.delete(connection); resolve();
  }));
  client.on('error', error => { if (!state.expectedEnd) state.error = error; });
  for (const packet of ['disconnect', 'kick_disconnect']) client.on(packet, value => { state.reason = JSON.stringify(value.reason); });
  client.on('keep_alive', packet => client.write('keep_alive', { keepAliveId: packet.keepAliveId }));
  client.on('login', packet => { state.joined = true; state.mode = packet.worldState?.gamemode; });
  client.on('position', packet => {
    state.position = { x: packet.x, y: packet.y, z: packet.z };
    state.positionRevision++;
    if (!state.teleports.has(packet.teleportId)) {
      state.teleports.add(packet.teleportId);
      client.write('teleport_confirm', { teleportId: packet.teleportId });
    }
  });
  client.on('map_chunk', packet => {
    try {
      const column = new Chunk({ x: packet.x, z: packet.z });
      column.load(packet.chunkData, true, false);
      state.columns.set(`${packet.x},${packet.z}`, column);
    } catch (error) { state.error = error; }
  });
  client.on('unload_chunk', packet => state.columns.delete(`${packet.chunkX},${packet.chunkZ}`));
  client.on('block_change', packet => state.changes.set(key(packet.location), packet.type));
  client.on('acknowledge_player_digging', packet => state.acks.add(packet.sequenceId));
  client.on('window_items', packet => {
    if (packet.windowId === 0) {
      for (const [slot, item] of packet.items.entries()) state.slots.set(slot, { item, revision: ++state.slotRevision });
    }
  });
  client.on('set_slot', packet => {
    if (packet.windowId === 0) state.slots.set(packet.slot, { item: packet.item, revision: ++state.slotRevision });
  });
  await until(() => state.joined && state.position && state.columns.has('0,0') && state.slots.has(36), `join ${name} and decode chunk/inventory`, [connection]);
  assert.equal(state.mode, 'creative', 'fixture joins in creative mode');
  return connection;
}

async function creative(connection, material) {
  const itemId = data.itemsByName[material]?.id;
  assert(Number.isInteger(itemId), `independent registry contains ${material} item`);
  const before = connection.state.slotRevision;
  connection.client.write('held_item_slot', { slotId: 0 });
  connection.client.write('set_creative_slot', {
    slot: 36, item: { itemCount: 1, itemId, addedComponentCount: 0, removedComponentCount: 0, components: [], removeComponents: [] },
  });
  await until(() => connection.state.slots.get(36)?.revision > before, `${material} creative-slot echo`, [connection]);
  assert.deepEqual(connection.state.slots.get(36).item, {
    itemCount: 1, itemId, addedComponentCount: 0, removedComponentCount: 0, components: [], removeComponents: [],
  });
}

async function moveNear(connection, target) {
  const wanted = { x: target.x + 0.5, y: 64, z: target.z + 2 };
  const before = connection.state.positionRevision;
  connection.client.write('chat_command', { command: `tp ${wanted.x} ${wanted.y} ${wanted.z}` });
  await until(() => connection.state.positionRevision > before, 'operator teleport within placement reach', [connection]);
  assert.deepEqual(connection.state.position, wanted);
  // Its matching teleport ACK is written by the position handler before any
  // subsequent inventory/look/placement packet; the next slot echo is a fence.
}

async function placeDoor(actor, observer, target, facing, yaw, cursor, hinge, material = 'oak_door') {
  await moveNear(actor, target);
  await creative(actor, material);
  await until(() => actor.state.columns.has(columnKey(target)) && observer.state.columns.has(columnKey(target)), 'both viewers have placement chunk', [actor, observer]);
  const support = { ...target, y: target.y - 1 };
  for (const connection of [actor, observer]) {
    const block = connection.state.columns.get(columnKey(support)).getBlock(local(support));
    assert.equal(block.name, 'grass_block', `${connection.state.username}: actual clicked support is the flatworld grass block`);
  }
  for (const half of ['lower', 'upper']) {
    const position = { ...target, y: target.y + (half === 'upper' ? 1 : 0) };
    assert.equal(actor.state.columns.get(columnKey(position)).getBlock(local(position)).name, 'air', 'fixture placement cell starts as air');
    assert(!actor.state.changes.has(key(position)) && !observer.state.changes.has(key(position)), 'each case uses untouched cells');
    expected.set(key(position), { position, ...goldenState(material, facing, hinge, half) });
  }
  actor.client.write('look', { yaw, pitch: 0, flags: { onGround: true } });
  const seq = ++sequence;
  actor.client.write('block_place', {
    hand: 0, location: support, direction: 1,
    cursorX: cursor, cursorY: 1, cursorZ: cursor,
    insideBlock: false, worldBorderHit: false, sequence: seq,
  });
  const halves = [key(target), key({ ...target, y: target.y + 1 })];
  await until(() => actor.state.acks.has(seq) && halves.every(value => actor.state.changes.has(value) && observer.state.changes.has(value)), `${material} ${facing} cursor=${cursor}: ACK and two halves for both viewers`, [actor, observer]);
  for (const value of halves) for (const connection of [actor, observer]) {
    const golden = expected.get(value);
    assert.equal(connection.state.changes.get(value), golden.id, `${connection.state.username}: state ID at ${value}`);
    assert.deepEqual(decode(connection.state.changes.get(value)), { name: golden.name, properties: golden.properties });
  }
  console.log(`PASS: ${material} facing=${facing} cursor=${cursor} hinge=${hinge}; lower/upper on actor and observer`);
}

async function restartAssertions(connection) {
  await until(() => [...expected.values()].every(value => connection.state.columns.has(columnKey(value.position))), `${connection.state.username}: persisted chunks decoded`, [connection]);
  for (const [value, golden] of expected) {
    const block = connection.state.columns.get(columnKey(golden.position)).getBlock(local(golden.position));
    assert.equal(block.stateId, golden.id, `${connection.state.username}: persisted state ID at ${value}`);
    assert.deepEqual({ name: block.name, properties: block.getProperties() }, { name: golden.name, properties: golden.properties });
  }
  console.log(`PASS: ${connection.state.username} decoded ${expected.size} exact persisted half-states from restarted-server chunks`);
}

async function main() {
  selfTest();
  await setup();
  await start('initial');
  const actor = await connect(username);
  const observer = await connect('DoorObserver');
  for (const [row, orientation] of orientations.entries()) for (const [column, cursor] of cursors.entries()) {
    await placeDoor(actor, observer, { x: 4 + column * 4, y: 64, z: 4 + row * 4 }, orientation.facing, orientation.yaw, cursor, orientation.hinges[column]);
  }
  // South's left cell is east of the target. The birch lower half must force
  // the next oak door right, despite its exact-center cursor tie choosing left.
  await placeDoor(actor, observer, { x: 8, y: 64, z: 20 }, 'south', 0, 0.5, 'left', 'birch_door');
  await placeDoor(actor, observer, { x: 7, y: 64, z: 20 }, 'south', 0, 0.5, 'right');
  await stop();
  console.log('PASS: first server completed SIGINT save/drain while both clients were connected');
  await start('restarted');
  const restoredActor = await connect(username);
  const restoredObserver = await connect('DoorObserver');
  await restartAssertions(restoredActor);
  await restartAssertions(restoredObserver);
  await stop();
  assert.equal(await bounded(hashBinary(), 'final binary SHA-256'), binaryHash);
  fs.writeFileSync(path.join(runDir, 'result.json'), JSON.stringify({
    version, protocol: 772, binary, binaryHash, port, doorPlacements: 14, persistedHalfStates: expected.size,
    observers: 2, gracefulProcessRuns: 2, paperRuntimeDifferential: false,
  }, null, 2) + '\n');
  console.log(`DOOR HINGE CLIENT TEST PASSED (${runDir})`);
}

try {
  if (process.argv.includes('--self-test')) selfTest();
  else await main();
} catch (error) {
  console.error(error.stack || error);
  if (runDir) console.error(`Artifacts: ${runDir}`);
  try { await stop(); } catch (cleanup) { console.error(`Cleanup: ${cleanup.message}`); }
  process.exitCode = 1;
}
