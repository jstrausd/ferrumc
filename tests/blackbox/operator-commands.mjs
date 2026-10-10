#!/usr/bin/env node
// D0: independent client graphs, live authorization, CLI recovery and restart.
// Native-only offline names and unsigned commands; no Paper differential claim.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import mc from 'minecraft-protocol';

const here = path.dirname(fileURLToPath(import.meta.url));
const binary = path.resolve(process.env.MC_SERVER_BIN || path.join(here, '../../server/target/debug/ferrumc'));
const runDir = fs.mkdtempSync(path.join(os.tmpdir(), 'ferrumc-operator-client-'));
const configPath = path.join(runDir, 'config.toml');
const connections = new Set();
let server, serverExit, serverError, logFile, port;

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
    if (server && (server.exitCode !== null || server.signalCode !== null))
      throw new Error(`Server exited during ${label}; inspect ${logFile}`);
    if (connection?.state.error) throw connection.state.error;
    if (connection?.state.ended) throw new Error(`Client ended during ${label}`);
    if (predicate()) return;
    if (Date.now() >= end) throw new Error(`Timeout: ${label}; inspect ${logFile}`);
    // Time is a failure bound; successful gates observe actual packets/logs.
    await new Promise(resolve => setTimeout(resolve, 10));
  }
}

function simplify(tag) {
  if (!tag || typeof tag !== 'object') return tag;
  if (tag.type === 'compound')
    return Object.fromEntries(Object.entries(tag.value).map(([key, value]) => [key, simplify(value)]));
  if (tag.type === 'list') return tag.value.value.map(value => simplify({ type: tag.value.type, value }));
  return Object.hasOwn(tag, 'value') ? tag.value : tag;
}

function text(component) {
  if (typeof component === 'string') return component;
  if (Array.isArray(component)) return component.map(text).join('');
  return component && typeof component === 'object' ? (component.text || '') + text(component.extra || []) : '';
}

function roots(connection) {
  const graph = connection.state.graph;
  assert(graph && graph.nodes[graph.rootIndex], 'a decoded command graph is required');
  return graph.nodes[graph.rootIndex].children.map(index => graph.nodes[index].extraNodeData?.name);
}

async function graph(connection, operator) {
  await until(() => {
    if (!connection.state.graph) return false;
    const visible = roots(connection);
    assert(visible.includes('spawn'), 'ordinary command is retained');
    for (const name of ['gamemode', 'deop']) assert.equal(visible.includes(name), visible.includes('op'));
    return visible.includes('op') === operator;
  }, `operator graph=${operator}`, connection);
}

async function connect(username, operator) {
  const client = mc.createClient({ host: '127.0.0.1', port, username, auth: 'offline', version: '1.21.8', keepAlive: false });
  const state = { graph: null, messages: [], mode: null, modeRevision: 0, position: false, ended: false, error: null };
  const connection = { client, state };
  connections.add(connection);
  connection.end = new Promise(resolve => client.once('end', () => {
    state.ended = true; connections.delete(connection); resolve();
  }));
  client.on('error', error => { state.error = error; });
  client.on('keep_alive', packet => client.write('keep_alive', { keepAliveId: packet.keepAliveId }));
  client.on('login', packet => {
    const mode = ['survival', 'creative', 'adventure', 'spectator'].indexOf(packet.worldState?.gamemode);
    if (mode < 0) state.error = new Error('Unknown join-game mode');
    else state.mode = mode;
  });
  client.on('position', packet => { state.position = true; client.write('teleport_confirm', { teleportId: packet.teleportId }); });
  client.on('declare_commands', packet => { state.graph = packet; });
  client.on('system_chat', packet => state.messages.push(text(simplify(packet.content))));
  client.on('game_state_change', packet => {
    if (packet.reason === 3 || packet.reason === 'change_game_mode') { state.mode = packet.gameMode; state.modeRevision++; }
  });
  await until(() => state.position && state.mode !== null, `join ${username}`, connection);
  await graph(connection, operator);
  return connection;
}

async function command(connection, value, wanted) {
  const start = connection.state.messages.length;
  connection.client.write('chat_command', { command: value });
  await until(() => connection.state.messages.slice(start).some(message => message.includes(wanted)), `/${value}: ${wanted}`, connection);
}

async function mode(connection, name, expected) {
  const revision = connection.state.modeRevision;
  await command(connection, `gamemode ${name}`, 'Game mode set to');
  await until(() => connection.state.modeRevision > revision && connection.state.mode === expected, `actual game mode ${name}`, connection);
  assert.equal(connection.state.mode, expected);
}

async function consoleCommand(value, wanted) {
  const offset = fs.statSync(logFile).size;
  await bounded(new Promise((resolve, reject) => server.stdin.write(`${value}\n`, error => error ? reject(error) : resolve())), 'console write');
  await until(() => fs.readFileSync(logFile).subarray(offset).toString('utf8').includes(wanted), `console ${value}`);
}

async function start(label) {
  logFile = path.join(runDir, `server-${label}.log`);
  serverError = null;
  const out = fs.openSync(logFile, 'w');
  try {
    server = spawn(binary, ['--config', configPath], {
      cwd: runDir, env: { ...process.env, RUST_LOG: 'info' }, stdio: ['pipe', out, out],
    });
    serverExit = new Promise(resolve => {
      server.once('error', error => { serverError = error; resolve({ error }); });
      server.once('exit', (code, signal) => resolve({ code, signal }));
    });
    server.stdin.on('error', error => { serverError = error; });
  } finally { fs.closeSync(out); }
  await until(() => fs.readFileSync(logFile, 'utf8').includes('listening for Minecraft clients'), `${label} readiness`, null, 60_000);
}

async function stop() {
  let failure;
  const ended = await Promise.allSettled([...connections].map(async connection => {
    connection.client.end('operator test stop');
    await bounded(connection.end, 'client disconnect');
  }));
  for (const result of ended) if (result.status === 'rejected') failure ||= result.reason;
  const child = server;
  // Client cleanup failures must still reach process cleanup. A forced exit
  // reaps this owned child but cannot turn a failed shutdown into a passing gate.
  try {
    assert(child && child.exitCode === null && child.signalCode === null, 'server must be running before stop');
    child.kill('SIGTERM');
    const result = await bounded(serverExit, 'graceful shutdown');
    assert.deepEqual(result, { code: 0, signal: null });
    assert(fs.readFileSync(logFile, 'utf8').includes('shutdown complete'));
  } catch (error) {
    failure ||= error;
    if (child && child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
    if (child) {
      try { await bounded(serverExit, 'forced cleanup', 5000); }
      catch (cleanup) { console.error(`Process cleanup failed: ${cleanup.message}`); }
    }
  }
  server = null;
  if (failure) throw failure;
}

async function main() {
  assert(fs.statSync(binary).isFile());
  const socket = net.createServer();
  await new Promise((resolve, reject) => { socket.once('error', reject); socket.listen(0, '127.0.0.1', resolve); });
  port = socket.address().port;
  await new Promise((resolve, reject) => socket.close(error => error ? reject(error) : resolve()));
  fs.writeFileSync(configPath, [
    `bind = ${JSON.stringify(`127.0.0.1:${port}`)}`,
    `world_dir = ${JSON.stringify(path.join(runDir, 'world'))}`,
    'dashboard_enabled = false', 'builtin_plugins = false', 'spawn_chunk_radius = 1',
    'view_distance = 2', 'ops = ["Admin"]', '',
  ].join('\n'));
  await start('initial');
  const admin = await connect('Admin', true);
  const member = await connect('Member', false);
  await command(member, 'op Member', 'permission denied');
  await command(admin, 'op Member', 'Member is now an operator');
  await graph(member, true);
  await mode(member, 'survival', 0);
  await mode(member, 'creative', 1);
  await command(admin, 'deop Member', 'Member is no longer an operator');
  await graph(member, false);
  await command(member, 'gamemode survival', 'permission denied');
  await command(member, 'op Member', 'permission denied');
  await command(admin, 'op Keeper', 'Keeper is now an operator');
  await command(admin, 'deop Admin', 'Admin is no longer an operator');
  await graph(admin, false);
  await command(admin, 'op Admin', 'permission denied');
  await stop();
  console.log('PASS: two clients receive live grants/revocations and actual game mode changes');

  await start('restarted');
  const oldAdmin = await connect('Admin', false);
  const oldMember = await connect('Member', false);
  const keeper = await connect('Keeper', true);
  await until(() => oldMember.state.modeRevision > 0, 'restored player mode event', oldMember);
  assert.equal(oldMember.state.mode, 1, 'denied survival command did not mutate the persisted mode');
  await command(oldAdmin, 'op Admin', 'permission denied');
  await consoleCommand('op Member', 'Member is now an operator');
  await graph(oldMember, true);
  await consoleCommand('deop Keeper', 'Keeper is no longer an operator');
  await graph(keeper, false);
  await command(keeper, 'op Keeper', 'permission denied');
  server.stdin.end();
  await mode(oldMember, 'survival', 0);
  await mode(oldMember, 'creative', 1);
  await stop();
  console.log('PASS: restart retains offline grant and config-seed revocation; CLI recovers access; stdin EOF leaves server usable');

  await start('console-restarted');
  const finalAdmin = await connect('Admin', false);
  const finalMember = await connect('Member', true);
  const finalKeeper = await connect('Keeper', false);
  await command(finalAdmin, 'op Admin', 'permission denied');
  await command(finalKeeper, 'op Keeper', 'permission denied');
  await mode(finalMember, 'survival', 0);
  await mode(finalMember, 'creative', 1);
  await stop();
  console.log('PASS: CLI grant and revoke remain effective after a second real process restart');
  console.log(`OPERATOR CLIENT TEST PASSED (${runDir})`);
}

try { await main(); }
catch (error) {
  console.error(error.stack || error);
  console.error(`Artifacts: ${runDir}`);
  try { if (server) await stop(); } catch (cleanup) { console.error(cleanup.message); }
  process.exitCode = 1;
}
