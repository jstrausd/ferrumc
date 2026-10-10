#!/usr/bin/env node
// I01 regression gate: a full bag must never discard or duplicate the cursor.
// All inventory and click packets use the independent Minecraft 1.21.8 codec.
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
const host = '127.0.0.1';
const username = 'CursorTester';
const binary = path.resolve(process.env.MC_SERVER_BIN || path.join(here, '../../server/target/debug/ferrumc'));
// Always make a new world; never erase or reuse a caller's existing directory.
const runDir = fs.mkdtempSync(path.join(os.tmpdir(), 'ferrumc-inventory-client-'));
const configPath = path.join(runDir, 'config.toml');
const data = mcData('1.21.8');
const registry = mcRegistry('1.21.8');
const Chunk = ChunkLoader(registry);
const Block = BlockLoader(registry);
const Vec3 = Vec3pkg.Vec3 || Vec3pkg.default || Vec3pkg;
const markerId = data.itemsByName.diamond.id;
const stoneId = data.itemsByName.stone.id;
const chestA = { x: 10, y: 64, z: 8 };
const chestB = { x: 10, y: 64, z: 10 };
const clients = new Set();
let port;
let server;
let serverExit;
let serverError;
let serverLog;
let sequence = 0;

function stack(itemId, itemCount) {
  return itemCount ? {
    itemCount, itemId, addedComponentCount: 0, removedComponentCount: 0,
    components: [], removeComponents: [],
  } : { itemCount: 0 };
}

const empty = () => stack(0, 0);
const count = (slots, id) => slots.reduce((total, item) => total + (item.itemId === id ? item.itemCount : 0), 0);
const key = position => `${position.x},${position.y},${position.z}`;

async function until(predicate, label, timeout = 20_000, connection) {
  const end = Date.now() + timeout;
  for (;;) {
    if (serverError) throw serverError;
    if (server && (server.exitCode !== null || server.signalCode !== null))
      throw new Error(`Server exited while waiting for ${label}; inspect ${serverLog}`);
    if (connection?.state.error) throw connection.state.error;
    if (connection?.state.ended) throw new Error(`Client disconnected while waiting for ${label}: ${connection.state.reason}`);
    if (predicate()) return;
    if (Date.now() >= end) throw new Error(`Timeout: ${label}`);
    // This interval only checks a packet/process predicate; it is not a fence.
    await new Promise(resolve => setTimeout(resolve, 10));
  }
}

async function bounded(promise, label, timeout = 30_000) {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`Timeout: ${label}`)), timeout);
    })]);
  } finally { clearTimeout(timer); }
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
  await until(() => fs.readFileSync(serverLog, 'utf8').includes('listening for Minecraft clients'), `${label} readiness`, 60_000);
}

async function stop() {
  if (!server) return;
  for (const connection of clients) connection.state.expectedEnd = true;
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
  await bounded(Promise.all([...clients].map(connection => connection.ended)), 'clients close after shutdown');
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
    'keep_alive_interval_ms = 10000', '',
  ].join('\n'));
}

async function connectOnce() {
  const state = {
    error: null, ended: false, reason: null, expectedEnd: false,
    position: null, graph: false, columns: new Map(), blocks: new Map(), acks: new Set(),
    windows: new Map(), active: 0, openRevision: 0, revision: 0,
    slotRevisions: new Map(), cursor: empty(),
  };
  const client = mc.createClient({ host, port, username, auth: 'offline', version: '1.21.8', keepAlive: false });
  const connection = { client, state };
  clients.add(connection);
  connection.ended = new Promise(resolve => client.once('end', reason => {
    state.ended = true;
    state.reason ||= reason;
    clients.delete(connection);
    resolve();
  }));
  client.on('error', error => { if (!state.expectedEnd) state.error = error; });
  client.on('kick_disconnect', packet => { state.reason = JSON.stringify(packet.reason); });
  client.on('disconnect', packet => { state.reason = JSON.stringify(packet.reason); });
  client.on('keep_alive', packet => client.write('keep_alive', { keepAliveId: packet.keepAliveId }));
  client.on('position', packet => {
    state.position = packet;
    client.write('teleport_confirm', { teleportId: packet.teleportId });
  });
  client.on('declare_commands', () => { state.graph = true; });
  client.on('map_chunk', packet => {
    try {
      const column = new Chunk({ x: packet.x, z: packet.z });
      column.load(packet.chunkData, true, false);
      state.columns.set(`${packet.x},${packet.z}`, column);
    } catch (error) { state.error = error; }
  });
  client.on('block_change', packet => { state.blocks.set(key(packet.location), packet.type); });
  client.on('acknowledge_player_digging', packet => { state.acks.add(packet.sequenceId); });
  client.on('open_window', packet => {
    if (packet.inventoryType !== 2) {
      state.error = new Error(`Expected generic_9x3 chest, got type ${packet.inventoryType}`);
      return;
    }
    state.active = packet.windowId;
    state.openRevision++;
  });
  client.on('window_items', packet => {
    const snapshot = { ...packet, revision: ++state.revision };
    state.windows.set(packet.windowId, snapshot);
    state.cursor = packet.carriedItem;
    if (packet.windowId === 0) state.active = 0;
    else {
      // A chest snapshot mirrors player indices 9..44. Keep window0 coherent.
      const player = state.windows.get(0);
      if (player) player.items.splice(9, 36, ...packet.items.slice(27));
    }
  });
  client.on('set_slot', packet => {
    const window = state.windows.get(packet.windowId);
    if (window && packet.slot >= 0) {
      window.items[packet.slot] = packet.item;
      window.stateId = packet.stateId;
      state.slotRevisions.set(`${packet.windowId}:${packet.slot}`, ++state.revision);
    }
  });
  client.on('set_cursor_item', packet => { state.cursor = packet.contents; });
  try {
    await until(() => state.position && state.graph && state.windows.has(0) && state.columns.has('0,0'), 'join, inventory and spawn chunk', 20_000, connection);
  } catch (error) {
    error.serverReason = state.reason;
    state.expectedEnd = true;
    client.end('failed admission cleanup');
    await bounded(connection.ended, 'failed admission disconnect', 5000);
    throw error;
  }
  assert.equal(state.windows.get(0).items.length, 46);
  return connection;
}

async function connect({ retryReserved = false } = {}) {
  const end = Date.now() + 20_000;
  for (;;) {
    try { return await connectOnce(); }
    catch (error) {
      // Leave-save still owns this UUID until durable cleanup completes. Only
      // its explicit admission rejection may be retried, each after socket-end.
      if (!retryReserved || !error.serverReason?.includes('player UUID is already active or finishing leave')) throw error;
      if (Date.now() >= end) throw new Error('UUID admission remained reserved after disconnect', { cause: error });
    }
  }
}

async function disconnect(connection) {
  connection.state.expectedEnd = true;
  connection.client.end('inventory cursor test');
  await bounded(connection.ended, 'client disconnect');
}

async function creative(connection, slot, item) {
  const before = connection.state.revision;
  connection.client.write('set_creative_slot', { slot, item });
  await until(() => (connection.state.slotRevisions.get(`0:${slot}`) || 0) > before,
    `creative slot ${slot} echo`, 10_000, connection);
  const received = connection.state.windows.get(0).items[slot];
  assert.equal(received.itemCount, item.itemCount);
  if (item.itemCount) assert.equal(received.itemId, item.itemId);
  return received;
}

async function fillBag(connection) {
  for (let slot = 9; slot < 45; slot++) await creative(connection, slot, stack(stoneId, 64));
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
  await until(() => connection.state.acks.has(seq) && connection.state.blocks.has(key(position)),
    'chest placement block update and sequence acknowledgement', 10_000, connection);
  assert.equal(Block.fromStateId(connection.state.blocks.get(key(position)), 0).name, 'chest');
}

async function open(connection, position) {
  const beforeOpen = connection.state.openRevision;
  const beforeContent = connection.state.revision;
  const seq = useBlock(connection, position);
  await until(() => {
    const state = connection.state;
    return state.openRevision > beforeOpen && state.acks.has(seq)
      && state.windows.get(state.active)?.revision > beforeContent;
  }, 'new chest screen, content and sequence acknowledgement', 10_000, connection);
  const snapshot = connection.state.windows.get(connection.state.active);
  assert.equal(snapshot.items.length, 63);
  return snapshot;
}

async function close(connection) {
  const before = connection.state.revision;
  assert(connection.state.active > 0, 'close requires a chest');
  connection.client.write('close_window', { windowId: connection.state.active });
  await until(() => connection.state.windows.get(0)?.revision > before, 'close window0 content', 10_000, connection);
  assert.equal(connection.state.active, 0);
}

function hashed(item) {
  return item.itemCount ? {
    itemId: item.itemId, itemCount: item.itemCount, components: [], removeComponents: [],
  } : null;
}

async function click(connection, slot, overrides = {}) {
  const snapshot = connection.state.windows.get(connection.state.active);
  const before = connection.state.revision;
  const oldSlot = snapshot.items[slot] || empty();
  const oldCursor = connection.state.cursor;
  // This scenario exchanges unlike or empty stacks, so vanilla's prediction is
  // an exact swap. Claims are complete 1.21.8 HashedSlot values; named live
  // components use empty hashes because the server computes its own outcome.
  connection.client.write('window_click', {
    windowId: snapshot.windowId, stateId: snapshot.stateId, slot,
    mouseButton: 0, mode: 0,
    changedSlots: slot >= 0 && slot < snapshot.items.length ? [{ location: slot, item: hashed(oldCursor) }] : [],
    cursorItem: hashed(oldSlot), ...overrides,
  });
  await until(() => {
    assert.equal(connection.state.active, snapshot.windowId, 'click must leave the current chest active');
    return connection.state.windows.get(snapshot.windowId)?.revision > before;
  }, `click slot ${slot} authoritative content`, 10_000, connection);
  return connection.state.windows.get(snapshot.windowId);
}

function checkpoint(connection, label, cursorCount, chestMarkers = 0, id = markerId) {
  const state = connection.state;
  const snapshot = state.windows.get(state.active);
  const bag = state.active === 0 ? snapshot.items.slice(9, 45) : snapshot.items.slice(27);
  assert.equal(bag.length, 36, `${label}: bag slot count`);
  assert(bag.every(item => item.itemId === stoneId && item.itemCount === 64), `${label}: every bag slot remains full`);
  assert.equal(count(bag, stoneId), 2304, `${label}: bag stone total`);
  assert.equal(count(bag, id), 0, `${label}: marker must not fit in bag`);
  assert.equal(state.cursor.itemCount, cursorCount, `${label}: retained cursor count`);
  if (cursorCount) assert.equal(state.cursor.itemId, id, `${label}: retained cursor id`);
  if (state.active !== 0) assert.equal(count(snapshot.items.slice(0, 27), id), chestMarkers, `${label}: chest marker total`);
  assert.equal(cursorCount + chestMarkers, id === markerId ? 13 : 7, `${label}: local marker conservation`);
}

async function seed(connection, item) {
  await creative(connection, 36, item);
  await open(connection, chestA);
  await click(connection, 54); // chest27 + player hotbar index36 - main index9
  await click(connection, 0);
  assert.equal(connection.state.cursor.itemCount, 0);
  assert.equal(connection.state.windows.get(connection.state.active).items[0].itemCount, item.itemCount);
  await close(connection);
  await fillBag(connection);
}

async function namedLive(connection) {
  const name = 'I01 live marker';
  const encoded = Buffer.from(name, 'utf8');
  // anonymous TAG_Compound {text: TAG_String}; all text is ASCII, so its UTF-8
  // bytes are also the Java Modified UTF-8 representation required by NBT.
  const nbt = Buffer.alloc(11 + encoded.length);
  nbt[0] = 10;
  nbt[1] = 8;
  nbt.writeUInt16BE(4, 2);
  nbt.write('text', 4);
  nbt.writeUInt16BE(encoded.length, 8);
  encoded.copy(nbt, 10);
  const id = data.itemsByName.emerald.id;
  const named = { ...stack(id, 7), addedComponentCount: 1, components: [{ type: 'custom_name', data: nbt }] };
  const echoed = await creative(connection, 36, named);
  assert.deepEqual(echoed.components, [{ type: 'custom_name', data: { type: 'compound', value: { text: { type: 'string', value: name } } } }]);
  await seed(connection, named);
  await open(connection, chestA);
  await click(connection, 0);
  const assertNamed = label => {
    checkpoint(connection, label, 7, 0, id);
    assert.deepEqual(connection.state.cursor.components, echoed.components, `${label}: custom name survives`);
  };
  assertNamed('named pickup');
  await close(connection);
  assertNamed('named close');
  await open(connection, chestA);
  assertNamed('named reopen');
  await open(connection, chestB);
  assertNamed('named chest switch');
  await click(connection, 0);
  checkpoint(connection, 'named deposit', 0, 7, id);
  assert.deepEqual(connection.state.windows.get(connection.state.active).items[0].components, echoed.components);
  await open(connection, chestA);
  assert.equal(count(connection.state.windows.get(connection.state.active).items.slice(0, 27), id), 0,
    'named source chest remains empty after switch');
  await open(connection, chestB);
  checkpoint(connection, 'named destination reopen', 0, 7, id);
  await click(connection, 0);
  // Give the named cursor one free bag slot, then verify close returns it with
  // its component before removing the setup item. It never crosses a restart.
  await creative(connection, 36, empty());
  await close(connection);
  const returned = connection.state.windows.get(0).items[36];
  assert.equal(returned.itemCount, 7);
  assert.equal(returned.itemId, id);
  assert.deepEqual(returned.components, echoed.components);
  assert.equal(connection.state.cursor.itemCount, 0);
  await creative(connection, 36, empty());
  console.log('PASS: named live stack retained through full-bag close, reopen and chest switch');
}

function persistedBlock(connection, position) {
  const column = connection.state.columns.get(`${position.x >> 4},${position.z >> 4}`);
  return column.getBlock(new Vec3(position.x & 15, position.y, position.z & 15)).name;
}

async function main() {
  await setup();
  await start('initial');
  let connection = await connect();
  const uuid = connection.client.uuid;
  connection.client.write('held_item_slot', { slotId: 0 });
  await creative(connection, 36, stack(data.itemsByName.chest.id, 2));
  await placeChest(connection, chestA);
  await placeChest(connection, chestB);
  await namedLive(connection);

  await seed(connection, stack(markerId, 13));
  await open(connection, chestA);
  await click(connection, 0);
  checkpoint(connection, 'pickup', 13);
  await close(connection);
  checkpoint(connection, 'close', 13);
  await open(connection, chestA);
  checkpoint(connection, 'reopen', 13);
  const oldWindow = connection.state.active;
  await open(connection, chestB);
  checkpoint(connection, 'chest switch', 13);
  // Repeated/delayed close is fenced by the next current-window resync, not a
  // sleep. Both must leave the new chest active and preserve the cursor.
  connection.client.write('close_window', { windowId: oldWindow });
  connection.client.write('close_window', { windowId: oldWindow });
  await click(connection, 0, { stateId: -1 });
  checkpoint(connection, 'delayed repeated close and stale click rejected', 13);
  await assert.rejects(connectOnce, error =>
    error.serverReason?.includes('player UUID is already active or finishing leave') === true,
  'a concurrent connection for the same UUID must be explicitly rejected');
  await click(connection, 0, { stateId: -1 });
  checkpoint(connection, 'duplicate UUID rejected, original session retained', 13);
  console.log('PASS: full bag, close/reopen, chest switch and stale packets conserve all 13 markers');
  console.log('PASS: concurrent UUID admission is rejected without disturbing the original cursor');

  await disconnect(connection);
  connection = await connect({ retryReserved: true });
  assert.equal(connection.client.uuid, uuid, 'rejoin uses identical offline UUID');
  checkpoint(connection, 'rejoin window0', 13);
  await open(connection, chestA);
  checkpoint(connection, 'rejoin chestA', 13);
  await open(connection, chestB);
  checkpoint(connection, 'rejoin chestB', 13);
  console.log('PASS: disconnect/rejoin restores the same cursor and full bag');

  // Shutdown with the chest still open exercises the live connection save path.
  await stop();
  await start('restored');
  connection = await connect();
  assert.equal(connection.client.uuid, uuid);
  checkpoint(connection, 'process restart window0', 13);
  assert.equal(persistedBlock(connection, chestA), 'chest');
  assert.equal(persistedBlock(connection, chestB), 'chest');
  await open(connection, chestA);
  checkpoint(connection, 'process restart chestA', 13);
  await open(connection, chestB);
  checkpoint(connection, 'process restart chestB', 13);
  const completedState = connection.state.windows.get(connection.state.active).stateId;
  await click(connection, 0);
  checkpoint(connection, 'exact-once deposit', 0, 13);
  await click(connection, 0, { stateId: completedState });
  checkpoint(connection, 'repeated stale deposit', 0, 13);
  await close(connection);
  await open(connection, chestA);
  const aCount = count(connection.state.windows.get(connection.state.active).items.slice(0, 27), markerId);
  assert.equal(aCount, 0, 'source chest is empty after deposit');
  await open(connection, chestB);
  checkpoint(connection, 'reopen exact-once chest', 0, 13);
  assert.equal(aCount + count(connection.state.windows.get(connection.state.active).items.slice(0, 27), markerId)
    + count(connection.state.windows.get(0).items, markerId) + connection.state.cursor.itemCount, 13,
  'both chests, all player slots and cursor contain exactly 13 markers');
  console.log('PASS: process restart retains cursor; destination chest gets exactly 13 items once');

  await stop();
  await start('deposited');
  connection = await connect();
  assert.equal(count(connection.state.windows.get(0).items, markerId), 0);
  assert.equal(connection.state.cursor.itemCount, 0);
  await open(connection, chestB);
  checkpoint(connection, 'deposited process restart', 0, 13);
  await open(connection, chestA);
  assert.equal(count(connection.state.windows.get(connection.state.active).items.slice(0, 27), markerId), 0);
  await disconnect(connection);
  await stop();
  console.log(`INVENTORY CURSOR CLIENT TEST PASSED (${runDir})`);
}

try { await main(); }
catch (error) {
  console.error(error.stack || error);
  console.error(`Artifacts: ${runDir}`);
  for (const connection of clients) {
    connection.state.expectedEnd = true;
    connection.client.end('test cleanup');
  }
  try { await stop(); } catch (cleanup) { console.error(cleanup.stack || cleanup); }
  process.exitCode = 1;
}
