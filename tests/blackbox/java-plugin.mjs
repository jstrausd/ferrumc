#!/usr/bin/env node
// Independent protocol decoder + an ordinary, unmodified Bukkit fixture JAR.
// This checks the implemented API subset, not broad Paper compatibility.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import mc from 'minecraft-protocol';

const here = path.dirname(fileURLToPath(import.meta.url));
const external = process.env.MC_JAVA_EXTERNAL === '1';
const host = process.env.MC_HOST || '127.0.0.1';
const runDir = fs.mkdtempSync(path.join(os.tmpdir(), 'ferrumc-java-client-'));
const logPath = path.join(runDir, 'server.log');
const clients = new Set();
let server;
let serverExit;
let port;

function requiredFile(name, fallback) {
  const value = process.env[name] || fallback;
  assert(value && fs.statSync(value).isFile(), `${name} must point to a built file`);
  return path.resolve(value);
}

async function until(predicate, label, timeout = 20_000) {
  const deadline = Date.now() + timeout;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error(`Timeout: ${label}`);
    if (server && (server.exitCode !== null || server.signalCode !== null))
      throw new Error(`Server exited before ${label}; inspect ${logPath}`);
    await new Promise(resolve => setTimeout(resolve, 20));
  }
}

async function deadline(promise, label, ms = 30_000) {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`Timeout: ${label}`)), ms);
    })]);
  } finally { clearTimeout(timer); }
}

async function freePort() {
  const socket = net.createServer();
  await new Promise((resolve, reject) => {
    socket.once('error', reject);
    socket.listen(0, host, resolve);
  });
  const result = socket.address().port;
  await new Promise((resolve, reject) => socket.close(error => error ? reject(error) : resolve()));
  return result;
}

// Minecraft's text component arrives as anonymous NBT from PrismarineJS.
function simplify(tag) {
  if (!tag || typeof tag !== 'object') return tag;
  if (tag.type === 'compound')
    return Object.fromEntries(Object.entries(tag.value).map(([key, value]) => [key, simplify(value)]));
  if (tag.type === 'list') return tag.value.value.map(value => simplify({ type: tag.value.type, value }));
  return Object.hasOwn(tag, 'value') ? tag.value : tag;
}

function text(component) {
  if (typeof component === 'string') {
    try { return text(JSON.parse(component)); } catch { return component; }
  }
  if (Array.isArray(component)) return component.map(text).join('');
  if (component && typeof component === 'object')
    return (component.text || '') + text(component.extra || []);
  return '';
}

async function connect(username) {
  const state = { messages: [], graph: null, suggestions: null, error: null, ended: false };
  const client = mc.createClient({ host, port, username, auth: 'offline', version: '1.21.8' });
  clients.add(client);
  client.on('error', error => { state.error = error; });
  client.on('end', () => { state.ended = true; clients.delete(client); });
  client.on('position', packet => client.write('teleport_confirm', { teleportId: packet.teleportId }));
  client.on('system_chat', packet => state.messages.push(text(simplify(packet.content))));
  client.on('declare_commands', packet => { state.graph = packet; });
  client.on('tab_complete', packet => { state.suggestions = packet; });
  await until(() => {
    if (state.error) throw state.error;
    if (state.ended) throw new Error(`${username} disconnected during join`);
    return state.graph && state.messages.includes('join:monitor');
  }, `${username} graph and join event messages`);
  assert.deepEqual(state.messages.filter(message => message.startsWith('join:')), ['join:lowest', 'join:monitor']);
  return { client, state };
}

async function command(connection, value, matches) {
  const start = connection.state.messages.length;
  connection.client.write('chat_command', { command: value });
  try {
    await until(() => {
      if (connection.state.error) throw connection.state.error;
      if (connection.state.ended) throw new Error('Client disconnected during command');
      return connection.state.messages.slice(start).some(matches);
    }, `response to /${value}`);
  } catch (error) {
    throw new Error(`${error.message}; received ${JSON.stringify(connection.state.messages.slice(start))}`, { cause: error });
  }
  return connection.state.messages.slice(start);
}

async function disconnect(connection) {
  connection.client.end('java fixture complete');
  await deadline(new Promise(resolve => {
    if (connection.state.ended) resolve();
    else connection.client.once('end', resolve);
  }), 'client disconnect');
}

async function start() {
  if (external) {
    port = Number(process.env.MC_PORT);
    assert(Number.isInteger(port) && port > 0 && port <= 65535, 'MC_PORT required in external mode');
    return;
  }
  const binary = requiredFile('MC_SERVER_BIN', path.join(here, '../../server/target/debug/ferrumc'));
  const java = process.env.FERRUMC_JAVA_EXECUTABLE || 'java';
  const jar = requiredFile('FERRUMC_JAVA_BRIDGE_JAR');
  const fixture = requiredFile('FERRUMC_JAVA_FIXTURE_JAR');
  port = await freePort();
  const plugins = path.join(runDir, 'java-plugins');
  fs.mkdirSync(plugins);
  fs.copyFileSync(fixture, path.join(plugins, 'fixture.jar'));
  const config = path.join(runDir, 'config.toml');
  fs.writeFileSync(config, [
    `bind = ${JSON.stringify(`${host}:${port}`)}`,
    'dashboard_enabled = false',
    'view_distance = 2',
    'spawn_chunk_radius = 1',
    'ops = ["JavaOperator"]',
    `java_plugins_dir = ${JSON.stringify(plugins)}`,
    `java_bridge_jar = ${JSON.stringify(jar)}`,
    `java_executable = ${JSON.stringify(java)}`,
    'java_bridge_timeout_ms = 10000',
    '',
  ].join('\n'));
  const out = fs.openSync(logPath, 'w');
  try {
    server = spawn(binary, ['--config', config], {
      cwd: runDir, env: { ...process.env, RUST_LOG: 'info' }, stdio: ['ignore', out, out],
    });
    serverExit = new Promise((resolve, reject) => {
      server.once('error', reject);
      server.once('exit', (code, signal) => resolve({ code, signal }));
    });
    // Observe spawn failures immediately while readiness polling owns diagnostics.
    serverExit.catch(() => {});
  } finally { fs.closeSync(out); }
  await until(() => fs.readFileSync(logPath, 'utf8').includes('listening for Minecraft clients'), 'server readiness', 60_000);
}

async function stop() {
  for (const client of clients) client.end('test cleanup');
  if (!server) return;
  if (server.exitCode === null && server.signalCode === null) server.kill('SIGTERM');
  try {
    const result = await deadline(serverExit, 'graceful server/Java shutdown');
    assert.equal(result.code, 0, `server exit: ${JSON.stringify(result)}`);
  } catch (error) {
    server.kill('SIGKILL');
    await deadline(serverExit, 'forced cleanup', 5000);
    throw error;
  }
}

async function main() {
  await start();
  const operator = await connect('JavaOperator');
  const guest = await connect('JavaGuest');
  const labels = connection => connection.state.graph.nodes.map(node => node.extraNodeData?.name);
  assert(labels(operator).includes('bridgefixture') && labels(operator).includes('bfixture'));
  assert(!labels(guest).includes('bridgefixture') && !labels(guest).includes('bfixture'));
  console.log('PASS: ordered join listeners and permission-filtered command graph');

  const expected = `fixture:bfixture:JavaOperator:${operator.client.uuid}:true:true:FerrumC:1.0.0`;
  await command(operator, 'bfixture', message => message === expected);
  // PrismarineJS's NBT parser treats MUTF-8 surrogate bytes as ordinary UTF-8.
  // Exercise non-ASCII BMP text here; the Java subprocess tests cover emoji RPC.
  await command(operator, 'bridgefixture unicode-bmp', message => message === 'Grüße Ω');
  const denied = await command(guest, 'bfixture', message => message.includes('permission'));
  assert(!denied.some(message => message.startsWith('fixture:')));
  operator.client.write('tab_complete', { transactionId: 17, text: '/bf' });
  await until(() => operator.state.suggestions?.transactionId === 17, 'alias tab completion');
  assert(operator.state.suggestions.matches.some(match => match.match === 'bfixture'));
  console.log('PASS: alias, canonical UUID, UTF-8 text, Rust permissions and completion');

  await command(operator, 'bridgefixture world', message => message.includes('does not implement'));
  await command(operator, 'bridgefixture', message => message.startsWith('fixture:bridgefixture:'));
  // This response fences the guest's own packet stream before the absence
  // assertion, so a delayed earlier message cannot hide behind disconnect.
  await command(guest, 'BFIXTURE', message => message.includes('permission'));
  assert(!guest.state.messages.some(message => message.startsWith('fixture:') || message === 'Grüße Ω'));
  console.log('PASS: unsupported World API reports an error; next command succeeds; messages stay with sender');
  await disconnect(guest);
  await disconnect(operator);
  await stop();
  if (!external) {
    const config = fs.readFileSync(path.join(runDir, 'java-plugins/BridgeFixture/config.yml'), 'utf8');
    assert.match(config, /enabled: 1\b/);
    assert.match(config, /disabled: 1\b/);
    assert.match(config, /join-count: 2\b/);
    assert.match(config, /quit-count: 2\b/);
    const logs = fs.readFileSync(logPath, 'utf8');
    assert.equal(logs.split('fixture stdout onDisable').length - 1, 1);
    assert(logs.includes('shutdown complete'));
    console.log('PASS: plugin config and exactly one onDisable after graceful shutdown');
  }
  console.log(`JAVA PLUGIN CLIENT TEST PASSED (${runDir})`);
}

try { await main(); }
catch (error) {
  console.error(error.stack || error);
  console.error(`Artifacts: ${runDir}`);
  try { await stop(); } catch (cleanup) { console.error(cleanup.message); }
  process.exitCode = 1;
}
