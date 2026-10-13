#!/usr/bin/env node
// Independent protocol/section decoder: initial ground arrives before position.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import mc from 'minecraft-protocol';
import Registry from 'prismarine-registry';
import ChunkLoader from 'prismarine-chunk';
import Vec3pkg from 'vec3';

const Vec3 = Vec3pkg.Vec3 || Vec3pkg;
const registry = Registry('1.21.8'), Chunk = ChunkLoader(registry);
const binary = path.resolve(process.env.MC_SERVER_BIN);
const dir = path.resolve(process.env.MC_RUN_DIR);
const port = Number(process.env.MC_PORT || 25571), dashboardPort = Number(process.env.MC_DASHBOARD_PORT || 25572);
const preset = process.env.MC_NATURAL_PRESET || 'ferrumc_natural_tree_preview';
const allInterfaces = process.env.MC_ALL_INTERFACES !== '0';
assert(!fs.existsSync(path.join(dir, 'world')), 'fresh private test world required');
fs.mkdirSync(dir, { recursive: true });
const config = path.join(dir, 'config.toml');
fs.writeFileSync(config, `bind = "127.0.0.1:${port}"\ndashboard_bind = "127.0.0.1:${dashboardPort}"\n` +
  `dashboard_enabled = ${allInterfaces}\nworld_dir = ${JSON.stringify(path.join(dir, 'world'))}\n` +
  'spawn_chunk_radius = 0\nview_distance = 1\nsimulation_distance = 1\n' +
  'builtin_plugins = false\nops = ["GroundProbe"]\n[world]\nseed = 0\n' + `preset = ${JSON.stringify(preset)}\n`);
const binaryHash = crypto.createHash('sha256').update(fs.readFileSync(binary)).digest('hex');
let server, exited, processError;
const clients = new Set(), cases = [];
const key = p => `${Math.floor(p.x / 16)},${Math.floor(p.z / 16)}`;
const local = p => new Vec3(((Math.floor(p.x) % 16) + 16) % 16, Math.floor(p.y), ((Math.floor(p.z) % 16) + 16) % 16);
async function until(predicate, label, c, timeout = 60000) {
  const deadline = Date.now() + timeout;
  while (!predicate()) {
    if (processError) throw processError;
    if (c?.error) throw c.error;
    if (c?.ended) throw new Error(`client ended during ${label}`);
    if (server?.exitCode !== null || server?.signalCode !== null) throw new Error(`server exited during ${label}`);
    if (Date.now() >= deadline) throw new Error(`timeout: ${label}`);
    await new Promise(resolve => setTimeout(resolve, 15));
  }
}
async function bounded(promise, label, ms = 30000) {
  let timer;
  try { return await Promise.race([promise, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`timeout: ${label}`)), ms); })]); }
  finally { clearTimeout(timer); }
}
async function start(round) {
  const log = path.join(dir, `server-${round}.log`), fd = fs.openSync(log, 'w');
  processError = null;
  try { server = spawn(binary, ['--config', config, ...(allInterfaces ? ['--bind-all-interfaces'] : [])], { cwd: dir, stdio: ['ignore', fd, fd] }); }
  finally { fs.closeSync(fd); }
  exited = new Promise(resolve => { server.once('error', e => { processError = e; resolve({ error: e.message }); }); server.once('exit', (code, signal) => resolve({ code, signal })); });
  await until(() => fs.readFileSync(log, 'utf8').includes('listening for Minecraft clients'), 'server ready', null, 120000);
  if (allInterfaces) {
    const listeners = execFileSync('ss', ['-H', '-ltn'], { encoding: 'utf8' });
    assert(listeners.includes(`0.0.0.0:${port}`), 'Minecraft binds wildcard despite loopback config');
    assert(listeners.includes(`0.0.0.0:${dashboardPort}`), 'dashboard binds wildcard despite loopback config');
    const response = await fetch(`http://127.0.0.1:${dashboardPort}/api/snapshot`);
    assert.equal(response.status, 200); assert.equal(typeof await response.json(), 'object');
    const refused = await fetch(`http://127.0.0.1:${dashboardPort}/api/snapshot`, { method: 'POST' });
    assert.equal(refused.status, 405, 'wildcard dashboard remains read-only');
  }
}
async function stop() {
  if (!server) return;
  for (const c of clients) { c.expectedEnd = true; c.client.socket?.destroy(); }
  clients.clear();
  const child = server;
  try { child.kill('SIGINT'); assert.deepEqual(await bounded(exited, 'graceful stop'), { code: 0, signal: null }); }
  finally {
    if (child.exitCode === null && child.signalCode === null) { child.kill('SIGKILL'); await bounded(exited, 'forced cleanup'); }
    server = null;
  }
}
async function connect(name, label) {
  const c = { columns: new Map(), order: [], revision: 0, times: 0, centre: null, initial: null };
  c.client = mc.createClient({ host: '127.0.0.1', port, username: name, version: '1.21.8', auth: 'offline', keepAlive: false });
  clients.add(c); c.done = new Promise(resolve => c.client.once('end', () => { c.ended = true; resolve(); }));
  c.client.on('error', e => { if (!c.expectedEnd) c.error = e; });
  c.client.on('kick_disconnect', p => { if (!c.expectedEnd) c.error = new Error(JSON.stringify(p)); });
  c.client.on('keep_alive', p => c.client.write('keep_alive', { keepAliveId: p.keepAliveId }));
  c.client.on('update_time', () => c.times++);
  c.client.on('update_view_position', p => { c.centre = { x: p.chunkX, z: p.chunkZ }; c.order.push({ name: 'centre', value: c.centre }); });
  c.client.on('map_chunk', p => {
    try { const chunk = new Chunk({ x: p.x, z: p.z }); chunk.load(p.chunkData, true, false); c.columns.set(`${p.x},${p.z}`, chunk); c.order.push({ name: 'chunk', x: p.x, z: p.z }); }
    catch (e) { c.error = e; }
  });
  c.client.on('position', p => {
    try {
      const position = { x: p.x, y: p.y, z: p.z };
      if (!c.initial) {
        assert(c.columns.has(key(position)), 'destination ground chunk must be decoded before the first teleport');
        assert.deepEqual(c.centre, { x: Math.floor(p.x / 16), z: Math.floor(p.z / 16) }, 'first centre must be actual destination');
        c.initial = position;
      }
      c.position = position; c.revision++; c.order.push({ name: 'position', value: position });
      c.client.write('teleport_confirm', { teleportId: p.teleportId });
      c.client.write('player_loaded', {});
      c.client.write('position', { ...position, flags: { onGround: true, hasHorizontalCollision: false } });
    } catch (e) { c.error = e; }
  });
  await until(() => c.initial && !c.error, label, c);
  const chunk = c.columns.get(key(c.initial)), feet = local(c.initial);
  const below = chunk.getBlock(new Vec3(feet.x, feet.y - 1, feet.z));
  assert(below && below.boundingBox === 'block' && !below.name.endsWith('_leaves'), 'initial feet have real terrain support');
  for (const dy of [0, 1]) assert(['air', 'short_grass'].includes(chunk.getBlock(new Vec3(feet.x, feet.y + dy, feet.z)).name), 'initial body clear');
  const times = c.times; await until(() => c.times >= times + 2, 'live ticks after join', c);
  assert.deepEqual(c.position, c.initial, 'stationary join did not acquire a corrective falling position');
  cases.push({ label, initial: c.initial, support: below.name, order: c.order.slice(0, c.order.findIndex(p => p.name === 'position') + 1), liveTicks: c.times });
  return c;
}
async function close(c) {
  c.expectedEnd = true; c.client.end(); await bounded(c.done, 'disconnect'); clients.delete(c);
}
async function teleport(c, position) {
  const revision = c.revision;
  c.client.write('chat_command', { command: `tp ${position.x} ${position.y} ${position.z}` });
  await until(() => c.revision > revision, 'operator teleport', c);
  assert.deepEqual(c.position, position);
}
try {
  await start(1);
  const first = await connect('GroundProbe', 'fresh join');
  await teleport(first, { x: 88.5, y: 200, z: 8.5 });
  await until(() => first.columns.has('5,0'), 'far natural column', first);
  const far = first.columns.get('5,0'); let farY;
  for (let y = 316; y >= -62; y--) {
    const block = far.getBlock(new Vec3(8, y, 8));
    if (block.boundingBox === 'block' && !block.name.endsWith('_log') && !block.name.endsWith('_leaves') &&
        ['air', 'short_grass'].includes(far.getBlock(new Vec3(8, y + 1, 8)).name) &&
        ['air', 'short_grass'].includes(far.getBlock(new Vec3(8, y + 2, 8)).name)) { farY = y + 1; break; }
  }
  assert(Number.isInteger(farY), 'far real terrain support');
  const saved = { x: 88.5, y: farY, z: 8.5 }; await teleport(first, saved);
  const times = first.times; await until(() => first.times >= times + 2, 'saved teleport tick fence', first);
  await close(first); await stop();
  await start(2);
  const fresh = await connect('FreshGround2', 'second new player after restart'); await close(fresh);
  const returned = await connect('GroundProbe', 'restored far player after cold restart');
  assert.deepEqual(returned.initial, saved, 'valid saved position is preserved');
  await close(returned); await stop();
  const result = { status: 'PASS_INITIAL_GROUND_BEFORE_POSITION', binaryHash, preset, cases, restoredPosition: saved,
    allInterfaceMinecraftAndDashboard: allInterfaces, readOnlyDashboard: allInterfaces,
    scope: 'Independent protocol ordering/decoded terrain support, live ticks, two fresh joins and far saved cold restart; no visual Minecraft-client/full movement physics acceptance' };
  fs.writeFileSync(path.join(dir, 'acceptance.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
} finally { await stop(); }
