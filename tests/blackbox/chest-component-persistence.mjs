#!/usr/bin/env node
// S02: independent 1.21.8 Slot decoding, survival chest exchanges and real restarts.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import mc from 'minecraft-protocol';
import mcData from 'minecraft-data';
import mcRegistry from 'prismarine-registry';
import ChunkLoader from 'prismarine-chunk';
import BlockLoader from 'prismarine-block';
import Vec3pkg from 'vec3';

const here = path.dirname(fileURLToPath(import.meta.url));
const binary = path.resolve(process.env.MC_SERVER_BIN || path.join(here, '../../server/target/debug/ferrumc'));
const runDir = fs.mkdtempSync(path.join(os.tmpdir(), 'ferrumc-chest-components-'));
const configPath = path.join(runDir, 'config.toml');
const host = '127.0.0.1';
const username = 'ChestCodecTester';
const data = mcData('1.21.8');
const registry = mcRegistry('1.21.8');
const Chunk = ChunkLoader(registry);
const Block = BlockLoader(registry);
const Vec3 = Vec3pkg.Vec3 || Vec3pkg.default || Vec3pkg;
// The third-party protocol defines all offsets, including NBT boundaries. Raw
// Slot slices preserve duplicate keys and NaN payloads that its JS tree loses.
const proto = mc.createDeserializer({ state: 'play', isServer: false, version: '1.21.8' }).proto;
const markerId = data.itemsByName.emerald.id;
const markerCount = 7;
const chestA = { x: 10, y: 64, z: 8 };
const chestB = { x: 10, y: 64, z: 10 };
const connections = new Set();
let port;
let server;
let serverExit;
let serverError;
let serverLog;
let sequence = 0;

const empty = () => ({ itemCount: 0 });
const stack = (itemId, itemCount) => itemCount ? {
  itemCount, itemId, addedComponentCount: 0, removedComponentCount: 0,
  components: [], removeComponents: [],
} : empty();
const key = position => `${position.x},${position.y},${position.z}`;
const count = items => items.reduce((sum, item) => sum + (item.itemId === markerId ? item.itemCount : 0), 0);

async function bounded(promise, label, timeout = 30_000) {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`Timeout: ${label}`)), timeout);
    })]);
  } finally { clearTimeout(timer); }
}

async function until(predicate, label, connection, timeout = 20_000) {
  const end = Date.now() + timeout;
  for (;;) {
    if (serverError) throw serverError;
    if (server && (server.exitCode !== null || server.signalCode !== null))
      throw new Error(`Server exited during ${label}; inspect ${serverLog}`);
    if (connection?.state.error) throw connection.state.error;
    if (connection?.state.ended) throw new Error(`Client ended during ${label}: ${connection.state.reason}`);
    if (predicate()) return;
    if (Date.now() >= end) throw new Error(`Timeout: ${label}`);
    // This timer polls a packet/process predicate; elapsed time is never a fence.
    await new Promise(resolve => setTimeout(resolve, 10));
  }
}

async function setup() {
  assert(fs.statSync(binary).isFile(), 'MC_SERVER_BIN must point to a built server');
  const socket = net.createServer();
  await new Promise((resolve, reject) => {
    socket.once('error', reject);
    socket.listen(0, host, resolve);
  });
  port = socket.address().port;
  await new Promise((resolve, reject) => socket.close(error => error ? reject(error) : resolve()));
  fs.writeFileSync(configPath, [
    `bind = ${JSON.stringify(`${host}:${port}`)}`,
    `world_dir = ${JSON.stringify(path.join(runDir, 'world'))}`,
    'dashboard_enabled = false', 'builtin_plugins = false',
    'view_distance = 2', 'spawn_chunk_radius = 1', 'spawn = [8.0, 64.0, 8.0]',
    `ops = [${JSON.stringify(username)}]`, 'keep_alive_interval_ms = 10000', '',
  ].join('\n'));
}

async function start(label) {
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
  await until(() => fs.readFileSync(serverLog, 'utf8').includes('listening for Minecraft clients'), `${label} readiness`, null, 60_000);
}

async function stop() {
  if (!server) return;
  for (const connection of connections) connection.state.expectedEnd = true;
  if (server.exitCode === null && server.signalCode === null) server.kill('SIGINT');
  let result;
  try { result = await bounded(serverExit, 'graceful server shutdown'); }
  catch (error) {
    server.kill('SIGKILL');
    await bounded(serverExit, 'forced server cleanup', 5000);
    throw error;
  } finally { server = null; }
  if (result.error) throw result.error;
  assert.equal(result.code, 0, `server exit: ${JSON.stringify(result)}`);
  await bounded(Promise.all([...connections].map(connection => connection.ended)), 'clients close after shutdown');
}

function slotSlices(name, raw) {
  let offset = 0;
  const read = type => {
    const value = proto.read(raw, offset, type);
    offset += value.size;
    return value.value;
  };
  const slot = () => {
    const start = offset;
    read('Slot');
    return Buffer.from(raw.subarray(start, offset));
  };
  read('varint'); // independent packet ID
  if (name === 'window_items') {
    read('ContainerID'); read('varint');
    const size = read('varint');
    const items = Array.from({ length: size }, slot);
    const cursor = slot();
    assert.equal(offset, raw.length, 'complete independently decoded window_items');
    return { items, cursor };
  }
  read('ContainerID'); read('varint'); read('i16');
  const item = slot();
  assert.equal(offset, raw.length, 'complete independently decoded set_slot');
  return { item };
}

async function connect() {
  const state = {
    error: null, ended: false, reason: '', expectedEnd: false,
    position: false, graph: false, columns: new Map(), blocks: new Map(), acks: new Set(),
    windows: new Map(), active: 0, chest: null, chests: new Map(), openRevision: 0,
    revision: 0, slotRevisions: new Map(), cursor: empty(), modeRevision: 0, mode: null,
  };
  const client = mc.createClient({ host, port, username, auth: 'offline', version: '1.21.8', keepAlive: false });
  const connection = { client, state };
  connections.add(connection);
  connection.ended = new Promise(resolve => client.once('end', reason => {
    state.ended = true; state.reason ||= reason; connections.delete(connection); resolve();
  }));
  client.on('error', error => { if (!state.expectedEnd) state.error = error; });
  client.on('kick_disconnect', packet => { state.reason = JSON.stringify(packet.reason); });
  client.on('disconnect', packet => { state.reason = JSON.stringify(packet.reason); });
  client.on('keep_alive', packet => client.write('keep_alive', { keepAliveId: packet.keepAliveId }));
  client.on('position', packet => { state.position = true; client.write('teleport_confirm', { teleportId: packet.teleportId }); });
  client.on('declare_commands', () => { state.graph = true; });
  client.on('map_chunk', packet => {
    try {
      const column = new Chunk({ x: packet.x, z: packet.z });
      column.load(packet.chunkData, true, false);
      state.columns.set(`${packet.x},${packet.z}`, column);
    } catch (error) { state.error = error; }
  });
  client.on('block_change', packet => state.blocks.set(key(packet.location), packet.type));
  client.on('acknowledge_player_digging', packet => state.acks.add(packet.sequenceId));
  client.on('open_window', packet => {
    if (packet.inventoryType !== 2) { state.error = new Error(`Expected generic_9x3 chest, got ${packet.inventoryType}`); return; }
    state.active = packet.windowId; state.openRevision++;
  });
  client.on('game_state_change', packet => {
    if (packet.reason === 'change_game_mode' || packet.reason === 3) { state.mode = packet.gameMode; state.modeRevision++; }
  });
  client.on('packet', (packet, metadata, raw) => {
    try {
      if (metadata.name === 'window_items') {
        const wire = slotSlices(metadata.name, raw);
        const snapshot = { ...packet, wire, revision: ++state.revision };
        state.windows.set(packet.windowId, snapshot);
        state.cursor = packet.carriedItem;
        if (packet.windowId === 0) { state.active = 0; state.chest = null; }
      } else if (metadata.name === 'set_slot') {
        const wire = slotSlices(metadata.name, raw);
        const snapshot = state.windows.get(packet.windowId);
        if (snapshot && packet.slot >= 0) {
          snapshot.items[packet.slot] = packet.item;
          snapshot.wire.items[packet.slot] = wire.item;
          snapshot.stateId = packet.stateId;
          state.slotRevisions.set(`${packet.windowId}:${packet.slot}`, ++state.revision);
        }
      }
    } catch (error) { state.error = error; }
  });
  await until(() => state.position && state.graph && state.windows.has(0) && state.columns.has('0,0'), 'join, inventory and spawn chunk', connection);
  assert.equal(state.windows.get(0).items.length, 46);
  return connection;
}

async function creative(connection, slot, item) {
  const before = connection.state.revision;
  connection.client.write('set_creative_slot', { slot, item });
  await until(() => (connection.state.slotRevisions.get(`0:${slot}`) || 0) > before, `creative echo ${slot}`, connection);
  const snapshot = connection.state.windows.get(0);
  assert.equal(snapshot.items[slot].itemCount, item.itemCount);
  if (item.itemCount) assert.equal(snapshot.items[slot].itemId, item.itemId);
  return { item: snapshot.items[slot], wire: snapshot.wire.items[slot] };
}

async function survival(connection) {
  const before = connection.state.modeRevision;
  connection.client.write('chat_command', { command: 'gamemode 0' });
  await until(() => connection.state.modeRevision > before, 'survival mode acknowledgement', connection);
  assert.equal(connection.state.mode, 0);
}

function useBlock(connection, position) {
  const seq = ++sequence;
  connection.client.write('block_place', {
    hand: 0, location: position, direction: 1,
    cursorX: 0.5, cursorY: 0.5, cursorZ: 0.5,
    insideBlock: false, worldBorderHit: false, sequence: seq,
  });
  return seq;
}

async function placeChest(connection, position) {
  const seq = useBlock(connection, { ...position, y: position.y - 1 });
  await until(() => connection.state.acks.has(seq) && connection.state.blocks.has(key(position)), 'chest placement and acknowledgement', connection);
  assert.equal(Block.fromStateId(connection.state.blocks.get(key(position)), 0).name, 'chest');
}

function recordChest(connection) {
  const state = connection.state;
  if (state.chest !== null) {
    state.chests.set(state.chest, structuredClone(state.windows.get(state.active).items.slice(0, 27)));
  }
}

async function open(connection, position) {
  recordChest(connection);
  const beforeOpen = connection.state.openRevision;
  const before = connection.state.revision;
  const seq = useBlock(connection, position);
  await until(() => connection.state.openRevision > beforeOpen && connection.state.acks.has(seq)
    && connection.state.windows.get(connection.state.active)?.revision > before, 'chest open, contents and acknowledgement', connection);
  const snapshot = connection.state.windows.get(connection.state.active);
  assert.equal(snapshot.items.length, 63);
  connection.state.chest = key(position);
  recordChest(connection);
  return snapshot;
}

async function click(connection, slot, overrides = {}) {
  const snapshot = connection.state.windows.get(connection.state.active);
  const before = connection.state.revision;
  connection.client.write('window_click', {
    windowId: snapshot.windowId, stateId: snapshot.stateId, slot,
    mouseButton: 0, mode: 0, changedSlots: [], cursorItem: null, ...overrides,
  });
  await until(() => connection.state.windows.get(snapshot.windowId)?.revision > before, `click ${slot} authoritative content`, connection);
  assert.equal(connection.state.active, snapshot.windowId);
  recordChest(connection);
  return connection.state.windows.get(snapshot.windowId);
}

async function close(connection) {
  recordChest(connection);
  const before = connection.state.revision;
  assert(connection.state.active > 0);
  connection.client.write('close_window', { windowId: connection.state.active });
  await until(() => connection.state.windows.get(0)?.revision > before, 'close window0 content', connection);
  assert.equal(connection.state.active, 0);
}

function marker() {
  const namedTag = (type, name, bytes) => {
    const encoded = Buffer.from(name, 'ascii');
    const prefix = Buffer.alloc(3 + encoded.length);
    prefix[0] = type; prefix.writeUInt16BE(encoded.length, 1); encoded.copy(prefix, 3);
    return Buffer.concat([prefix, bytes]);
  };
  const name = 'S02 exact chest marker';
  const encoded = Buffer.from(name, 'ascii');
  const stringValue = Buffer.alloc(2 + encoded.length);
  stringValue.writeUInt16BE(encoded.length); encoded.copy(stringValue, 2);
  const customName = Buffer.concat([Buffer.from([10]), namedTag(8, 'text', stringValue), Buffer.from([0])]);
  const customData = Buffer.concat([
    Buffer.from([10]),
    namedTag(5, 'float_nan', Buffer.from('7fc12345', 'hex')),
    namedTag(5, 'float_negative_zero', Buffer.from('80000000', 'hex')),
    namedTag(6, 'double_nan', Buffer.from('7ff8123456789abc', 'hex')),
    namedTag(6, 'double_negative_zero', Buffer.from('8000000000000000', 'hex')),
    namedTag(3, 'duplicate', Buffer.from('00000007', 'hex')),
    namedTag(3, 'duplicate', Buffer.from('00000008', 'hex')),
    Buffer.from([0]),
  ]);
  return {
    name, customData,
    item: {
      ...stack(markerId, markerCount), addedComponentCount: 3,
      components: [
        { type: 'custom_data', data: customData },
        { type: 'damage', data: Buffer.from([3]) },
        { type: 'custom_name', data: customName },
      ],
      removedComponentCount: 1, removeComponents: [{ type: 'unbreakable' }],
    },
  };
}

function checkCreativeMarker(received, fixture) {
  assert.equal(received.item.itemCount, markerCount);
  assert.equal(received.item.itemId, markerId);
  assert.equal(received.item.addedComponentCount, 3);
  assert.equal(received.item.removedComponentCount, 1);
  assert.deepEqual(received.item.removeComponents, [{ type: 'unbreakable' }]);
  const components = new Map(received.item.components.map(component => [component.type, component.data]));
  assert.equal(components.get('damage'), 3);
  assert.equal(components.get('custom_name').value.text.value, fixture.name);
  let offset = 0;
  const read = type => { const parsed = proto.read(received.wire, offset, type); offset += parsed.size; return parsed; };
  read('varint'); read('varint'); const additions = read('varint').value; read('varint');
  let exactData = false;
  for (let index = 0; index < additions; index++) {
    const start = offset;
    const component = read('SlotComponent').value;
    if (component.type === 'custom_data') {
      const typeBytes = proto.read(received.wire, start, 'SlotComponentType').size;
      assert.deepEqual(received.wire.subarray(start + typeBytes, offset), fixture.customData,
        'creative acceptance preserves supplied NBT order, duplicate keys and exact float bits');
      exactData = true;
    }
  }
  assert(exactData, 'custom_data component received');
}

function exact(snapshot, location, expected, label) {
  const item = location === 'cursor' ? snapshot.carriedItem : snapshot.items[location];
  const wire = location === 'cursor' ? snapshot.wire.cursor : snapshot.wire.items[location];
  assert.equal(item.itemId, markerId, `${label}: marker ID`);
  assert.equal(item.itemCount, markerCount, `${label}: exact count`);
  assert.deepEqual(wire, expected, `${label}: complete raw Slot bytes preserve every component, removal and NBT bit`);
}

function conserve(connection, label) {
  recordChest(connection);
  const state = connection.state;
  const snapshot = state.windows.get(state.active);
  assert(state.chests.has(key(chestA)) && state.chests.has(key(chestB)), `${label}: both chests observed`);
  const playerItems = state.active === 0 ? snapshot.items : state.windows.get(0).items.slice(0, 9).concat(snapshot.items.slice(27), state.windows.get(0).items[45]);
  const total = [...state.chests.values()].reduce((sum, items) => sum + count(items), 0)
    + count(playerItems) + count([snapshot.carriedItem]);
  assert.equal(total, markerCount, `${label}: both chests, all 46 player slots and cursor contain exactly seven markers`);
}

function persistedChest(connection, position) {
  const column = connection.state.columns.get(`${position.x >> 4},${position.z >> 4}`);
  assert.equal(column.getBlock(new Vec3(position.x & 15, position.y, position.z & 15)).name, 'chest');
}

async function main() {
  await setup();
  await start('initial');
  let connection = await connect();
  connection.client.write('held_item_slot', { slotId: 0 });
  await creative(connection, 36, stack(data.itemsByName.chest.id, 2));
  await placeChest(connection, chestA); await placeChest(connection, chestB);
  await creative(connection, 36, empty());
  assert.equal(count((await open(connection, chestA)).items), 0);
  assert.equal(count((await open(connection, chestB)).items), 0);
  await close(connection);
  const fixture = marker();
  const received = await creative(connection, 36, fixture.item);
  checkCreativeMarker(received, fixture);
  const expected = Buffer.from(received.wire);
  await survival(connection);
  await open(connection, chestA);
  exact(await click(connection, 54), 'cursor', expected, 'survival pickup from hotbar');
  exact(await click(connection, 0), 0, expected, 'survival deposit into source chest');
  conserve(connection, 'initial deposit');
  console.log('PASS S02: survival chest accepts name, damage, removal patch and exact NBT NaN/zero/duplicate-key bytes');
  await stop();

  await start('first-restored');
  connection = await connect();
  await survival(connection);
  assert.equal(count(connection.state.windows.get(0).items), 0);
  assert.equal(connection.state.cursor.itemCount, 0);
  persistedChest(connection, chestA); persistedChest(connection, chestB);
  exact(await open(connection, chestA), 0, expected, 'source chest after process restart');
  assert.equal(count((await open(connection, chestB)).items.slice(0, 27)), 0);
  conserve(connection, 'first process restart');
  await open(connection, chestA);
  exact(await click(connection, 0), 'cursor', expected, 'survival withdrawal after process restart');
  assert.equal(count(connection.state.windows.get(connection.state.active).items.slice(0, 27)), 0);
  exact(await click(connection, 54), 54, expected, 'source withdrawal into hotbar');
  exact(await open(connection, chestB), 54, expected, 'hotbar across chest switch');
  exact(await click(connection, 54), 'cursor', expected, 'destination pickup from hotbar');
  const staleState = connection.state.windows.get(connection.state.active).stateId;
  exact(await click(connection, 13), 13, expected, 'deposit into destination chest');
  exact(await click(connection, 13, { stateId: staleState }), 13, expected, 'stale repeated transfer refused');
  assert.equal(connection.state.cursor.itemCount, 0);
  conserve(connection, 'exactly-once transfer');
  console.log('PASS S02: first restart and survival transfer preserve exact components without duplication');
  await stop();

  await start('second-restored');
  connection = await connect();
  await survival(connection);
  assert.equal(count(connection.state.windows.get(0).items), 0);
  assert.equal(connection.state.cursor.itemCount, 0);
  assert.equal(count((await open(connection, chestA)).items.slice(0, 27)), 0);
  exact(await open(connection, chestB), 13, expected, 'destination chest after second process restart');
  conserve(connection, 'second process restart');
  exact(await click(connection, 13), 'cursor', expected, 'destination withdrawal after second restart');
  exact(await click(connection, 54), 54, expected, 'withdrawal into player hotbar');
  assert.equal(connection.state.cursor.itemCount, 0);
  conserve(connection, 'both chests empty after withdrawal');
  await close(connection);
  exact(connection.state.windows.get(0), 36, expected, 'window0 after close');
  conserve(connection, 'closed survival screen');
  assert.equal(count((await open(connection, chestB)).items.slice(0, 27)), 0);
  conserve(connection, 'empty destination reopened');
  await stop();
  console.log('PASS S02: second restart, withdrawal into hotbar and empty chests conserve exactly seven complete markers');
  console.log(`CHEST COMPONENT PERSISTENCE CLIENT TEST PASSED (${runDir})`);
}

try { await main(); }
catch (error) {
  console.error(error.stack || error);
  console.error(`Artifacts: ${runDir}`);
  for (const connection of connections) { connection.state.expectedEnd = true; connection.client.end('test cleanup'); }
  try { await stop(); } catch (cleanup) { console.error(cleanup.stack || cleanup); }
  process.exitCode = 1;
}
