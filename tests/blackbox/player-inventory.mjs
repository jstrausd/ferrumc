#!/usr/bin/env node
// I03: independent 1.21.8 codec, real Rust process and authoritative window-0 clicks.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import mc from 'minecraft-protocol';
import mcData from 'minecraft-data';

const here = path.dirname(fileURLToPath(import.meta.url));
const binary = path.resolve(process.env.MC_SERVER_BIN || path.join(here, '../../server/target/debug/ferrumc'));
const runDir = fs.mkdtempSync(path.join(os.tmpdir(), 'ferrumc-player-inventory-'));
const logFile = path.join(runDir, 'server.log');
const configPath = path.join(runDir, 'config.toml');
const data = mcData('1.21.8');
const emerald = data.itemsByName.emerald.id;
const stone = data.itemsByName.stone.id;
const username = 'InventoryTester';
const connections = new Set();
let server;
let serverExit;
let serverError;
let port;

const empty = () => ({ itemCount: 0 });
const stack = (id, count) => count ? {
  itemCount: count, itemId: id, addedComponentCount: 0, removedComponentCount: 0,
  components: [], removeComponents: [],
} : empty();

async function bounded(promise, label, ms = 30_000) {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`Timeout: ${label}`)), ms);
    })]);
  } finally { clearTimeout(timer); }
}

async function until(predicate, label, connection, ms = 20_000) {
  const end = Date.now() + ms;
  for (;;) {
    if (serverError) throw serverError;
    if (server?.exitCode !== null || server?.signalCode !== null)
      throw new Error(`Server exited during ${label}; inspect ${logFile}`);
    if (connection?.state.error) throw connection.state.error;
    if (connection?.state.ended) throw new Error(`Client ended during ${label}: ${connection.state.reason}`);
    if (predicate()) return;
    if (Date.now() >= end) throw new Error(`Timeout: ${label}; inspect ${logFile}`);
    // Poll only observed packets/process state; never use time as a transaction fence.
    await new Promise(resolve => setTimeout(resolve, 10));
  }
}

async function setup() {
  assert(fs.statSync(binary).isFile());
  const socket = net.createServer();
  await new Promise((resolve, reject) => {
    socket.once('error', reject);
    socket.listen(0, '127.0.0.1', resolve);
  });
  port = socket.address().port;
  await new Promise((resolve, reject) => socket.close(error => error ? reject(error) : resolve()));
  fs.writeFileSync(configPath, [
    `bind = ${JSON.stringify(`127.0.0.1:${port}`)}`,
    `world_dir = ${JSON.stringify(path.join(runDir, 'world'))}`,
    'dashboard_enabled = false', 'builtin_plugins = false', 'spawn_chunk_radius = 1',
    'view_distance = 2', `ops = [${JSON.stringify(username)}]`, '',
  ].join('\n'));
  const out = fs.openSync(logFile, 'w');
  try {
    server = spawn(binary, ['--config', configPath], {
      cwd: runDir, env: { ...process.env, RUST_LOG: 'info' }, stdio: ['ignore', out, out],
    });
    serverExit = new Promise(resolve => {
      server.once('error', error => { serverError = error; resolve({ error }); });
      server.once('exit', (code, signal) => resolve({ code, signal }));
    });
  } finally { fs.closeSync(out); }
  await until(() => fs.readFileSync(logFile, 'utf8').includes('listening for Minecraft clients'), 'readiness', null, 60_000);
}

async function connect(name) {
  const client = mc.createClient({ host: '127.0.0.1', port, username: name, auth: 'offline', version: '1.21.8', keepAlive: false });
  const state = { error: null, ended: false, expectedEnd: false, reason: '', view: null, revision: 0, slots: new Map(), equipment: [], modeRevision: 0, mode: null, position: false };
  const connection = { client, state };
  connections.add(connection);
  connection.end = new Promise(resolve => client.once('end', reason => {
    state.ended = true; state.reason ||= reason; connections.delete(connection); resolve();
  }));
  client.on('error', error => { if (!state.expectedEnd) state.error = error; });
  client.on('kick_disconnect', packet => { state.reason = JSON.stringify(packet.reason); });
  client.on('keep_alive', packet => client.write('keep_alive', { keepAliveId: packet.keepAliveId }));
  client.on('position', packet => { state.position = true; client.write('teleport_confirm', { teleportId: packet.teleportId }); });
  client.on('window_items', packet => {
    if (packet.windowId !== 0) { state.error = new Error(`Unexpected window ${packet.windowId}`); return; }
    state.view = packet; state.revision++;
  });
  client.on('set_slot', packet => {
    state.slots.set(packet.slot, { item: packet.item, revision: ++state.revision });
    if (state.view) { state.view.items[packet.slot] = packet.item; state.view.stateId = packet.stateId; }
  });
  client.on('entity_equipment', packet => state.equipment.push(packet));
  client.on('game_state_change', packet => {
    if (packet.reason === 'change_game_mode' || packet.reason === 3) { state.mode = packet.gameMode; state.modeRevision++; }
  });
  await until(() => state.position && state.view, `join ${name}`, connection);
  assert.equal(state.view.items.length, 46);
  return connection;
}

async function creative(connection, slot, item) {
  const before = connection.state.revision;
  connection.client.write('set_creative_slot', { slot, item });
  await until(() => connection.state.slots.get(slot)?.revision > before, `creative echo ${slot}`, connection);
  return connection.state.slots.get(slot).item;
}

async function mode(connection, value) {
  const before = connection.state.modeRevision;
  connection.client.write('chat_command', { command: `gamemode ${value}` });
  await until(() => connection.state.modeRevision > before, `mode ${value}`, connection);
  assert.equal(connection.state.mode, value);
}

async function click(connection, slot, button = 0, overrides = {}) {
  const before = connection.state.revision;
  connection.client.write('window_click', {
    windowId: 0, stateId: connection.state.view.stateId, slot,
    mouseButton: button, mode: 0, changedSlots: [], cursorItem: null, ...overrides,
  });
  await until(() => connection.state.revision > before, `click ${slot}/${button}`, connection);
  return connection.state.view;
}

async function close(connection) {
  const before = connection.state.revision;
  connection.client.write('close_window', { windowId: 0 });
  await until(() => connection.state.revision > before, 'close window0', connection);
  return connection.state.view;
}

function counts(view) {
  return view.items.concat(view.carriedItem).reduce((sum, item) => sum + (item.itemId === emerald ? item.itemCount : 0), 0);
}

function expect(view, slotCount, cursorCount, slot = 9) {
  assert.equal(view.items[slot].itemCount, slotCount);
  assert.equal(view.carriedItem.itemCount, cursorCount);
  assert.equal(counts(view), 13, 'all thirteen marker items must be conserved');
}

async function scenario() {
  const viewer = await connect('InventoryViewer');
  const player = await connect(username);
  const text = 'I03 named marker';
  const raw = Buffer.alloc(11 + text.length);
  raw[0] = 10; raw[1] = 8; raw.writeUInt16BE(4, 2); raw.write('text', 4);
  raw.writeUInt16BE(text.length, 8); raw.write(text, 10);
  const named = { ...stack(emerald, 13), addedComponentCount: 1, components: [{ type: 'custom_name', data: raw }] };
  const echo = await creative(player, 9, named);
  const components = echo.components;
  await mode(player, 0);
  let view = await click(player, 9, 1);
  expect(view, 6, 7);
  assert.deepEqual(view.items[9].components, components);
  assert.deepEqual(view.carriedItem.components, components);
  view = await click(player, 10, 1);
  assert.equal(view.items[10].itemCount, 1);
  assert.equal(view.carriedItem.itemCount, 6);
  const oldState = view.stateId;
  const equipmentStart = viewer.state.equipment.length;
  view = await click(player, 45);
  assert.equal(view.items[45].itemCount, 6);
  assert.equal(view.carriedItem.itemCount, 0);
  assert.deepEqual(view.items[45].components, components);
  await until(() => viewer.state.equipment.slice(equipmentStart).some(packet => packet.equipments.some(entry => entry.slot === 1 && entry.item.itemId === emerald && entry.item.itemCount === 6)), 'offhand equipment broadcast', viewer);
  const before = structuredClone(view);
  view = await click(player, 45, 0, { stateId: oldState });
  assert.deepEqual(view.items, before.items, 'stale state must not pick up offhand');
  assert.deepEqual(view.carriedItem, before.carriedItem);
  view = await click(player, 45);
  for (const [slot, overrides] of [[0, {}], [5, {}], [6, {}], [7, {}], [8, {}], [-999, {}], [46, {}], [9, { mode: 1 }], [9, { mode: 5 }], [9, { windowId: 99 }]]) {
    const old = structuredClone(view);
    view = await click(player, slot, 0, overrides);
    assert.deepEqual(view.items, old.items, 'unsupported operation must resync');
    assert.deepEqual(view.carriedItem, old.carriedItem);
    assert.notEqual(view.stateId, old.stateId, 'resync needs a fresh state');
    assert.equal(counts(view), 13);
  }
  view = await click(player, 1, 1);
  assert.equal(view.items[1].itemCount, 1);
  assert.equal(view.carriedItem.itemCount, 5);
  view = await close(player);
  assert.equal(view.items[1].itemCount, 0);
  assert.equal(view.carriedItem.itemCount, 0);
  assert.equal(counts(view), 13);
  const afterClose = structuredClone(view.items);
  view = await close(player);
  assert.deepEqual(view.items, afterClose, 'duplicate close conserves inventory');
  await mode(player, 3);
  view = await click(player, 9);
  assert.deepEqual(view.items, afterClose, 'spectator cannot mutate');
  assert.equal(view.carriedItem.itemCount, 0);
  await mode(player, 1);
  for (let slot = 9; slot < 45; slot++) await creative(player, slot, stack(stone, 64));
  await creative(player, 1, named);
  await creative(player, 2, stack(emerald, 3));
  await mode(player, 0);
  view = await click(player, 1);
  assert.equal(view.carriedItem.itemCount, 13);
  const fullBag = structuredClone(view.items);
  for (let i = 0; i < 2; i++) {
    view = await close(player);
    assert.deepEqual(view.items, fullBag, 'full bag retains crafting input');
    assert.equal(view.carriedItem.itemCount, 13, 'full bag retains exact cursor');
    assert.deepEqual(view.carriedItem.components, components);
  }
  console.log('PASS I03: survival left/right, components, state/window gates, equipment, spectator, close0 and full-bag conservation');
}

async function stop() {
  if (!server) return;
  for (const connection of connections) connection.state.expectedEnd = true;
  if (server.exitCode === null && server.signalCode === null) server.kill('SIGINT');
  let result;
  try { result = await bounded(serverExit, 'graceful stop'); }
  catch (error) { server.kill('SIGKILL'); await bounded(serverExit, 'forced cleanup', 5000); throw error; }
  finally { server = null; }
  if (result.error) throw result.error;
  assert.equal(result.code, 0, 'server stopped successfully');
  await bounded(Promise.all([...connections].map(connection => connection.end)), 'client cleanup');
  console.log(`Artifacts: ${runDir}`);
}

let failed = false;
try { await setup(); await scenario(); }
catch (error) { failed = true; console.error(error); }
finally { try { await stop(); } catch (error) { failed = true; console.error(error); } }
if (failed) process.exitCode = 1;
