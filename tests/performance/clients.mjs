#!/usr/bin/env node
// Bounded standing-player workload for process-baseline.py. Reuse the pinned
// independent protocol client; this creates no Rust-specific packet oracle.
import { createRequire } from 'node:module';
import readline from 'node:readline';
import { performance } from 'node:perf_hooks';

const require = createRequire(new URL('../blackbox/package.json', import.meta.url));
const mc = require('minecraft-protocol');
const [host, portText, viewText] = process.argv.slice(2);
const port = Number(portText);
const view = Number(viewText);
if (!host || !Number.isInteger(port) || port < 1 || port > 65535 ||
    !Number.isInteger(view) || view < 0 || view > 8) {
  throw new Error('Usage: clients.mjs HOST PORT VIEW_DISTANCE(0..8)');
}
const expectedChunks = (2 * view + 1) ** 2;
const peers = [];
let closing = false;

function deadline(promise, label, milliseconds = 30_000) {
  let timer;
  return Promise.race([promise, new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`Timeout: ${label}`)), milliseconds);
  })]).finally(() => clearTimeout(timer));
}

function connect(index) {
  const started = performance.now();
  const username = `PerfPlayer${index}`;
  const client = mc.createClient({ host, port, username, auth: 'offline', version: '1.21.8' });
  const peer = { client, username, chunks: new Set(), positionMs: null, graphMs: null,
    allChunksMs: null, expectedEnd: false, ended: false };
  peers.push(peer);
  const joined = new Promise((resolve, reject) => {
    const check = () => {
      if (peer.positionMs !== null && peer.graphMs !== null && peer.chunks.size === expectedChunks) {
        peer.allChunksMs = performance.now() - started;
        resolve({ username, position_ms: peer.positionMs, graph_ms: peer.graphMs,
          full_view_ms: peer.allChunksMs, chunks: peer.chunks.size });
      }
    };
    client.on('position', packet => {
      client.write('teleport_confirm', { teleportId: packet.teleportId });
      peer.positionMs ??= performance.now() - started;
      check();
    });
    client.on('declare_commands', () => {
      peer.graphMs ??= performance.now() - started;
      check();
    });
    client.on('map_chunk', packet => {
      if (closing || peer.expectedEnd) return;
      const coordinate = `${packet.x},${packet.z}`;
      if (!peer.chunks.has(coordinate) && peer.chunks.size >= expectedChunks) {
        const error = new Error(`${username}: unexpected extra chunks`);
        reject(error);
        // The join promise may already be fulfilled; fail the running workload
        // as well, and refuse insertion so retained coordinates stay bounded.
        void fail(error);
        return;
      }
      peer.chunks.add(coordinate);
      check();
    });
    client.on('error', error => {
      reject(error);
      if (!peer.expectedEnd) void fail(error);
    });
    client.on('end', reason => {
      peer.ended = true;
      if (!peer.expectedEnd) {
        const error = new Error(`${username}: unexpected disconnect: ${reason}`);
        reject(error);
        void fail(error);
      }
    });
  });
  return deadline(joined, `${username} graph, position and ${expectedChunks} chunks`);
}

async function close() {
  closing = true;
  const ended = peers.map(peer => new Promise(resolve => {
    peer.expectedEnd = true;
    if (peer.ended) return resolve();
    peer.client.once('end', resolve);
    peer.client.end('performance workload cleanup');
  }));
  try { await deadline(Promise.all(ended), 'client cleanup', 5000); }
  finally { for (const peer of peers) peer.client.socket?.destroy(); }
}

async function fail(error) {
  if (closing) return;
  console.error(error.stack || String(error));
  try { await close(); } finally { process.exit(1); }
}

async function command(line) {
  const value = JSON.parse(line);
  if (value.op === 'stop') {
    await close();
    console.log(JSON.stringify({ stopped: true, clients: peers.length }));
    process.exit(0);
  }
  if (value.op !== 'clients' || !Number.isInteger(value.count) ||
      value.count < peers.length || value.count > 8) {
    throw new Error('Expected monotonically increasing clients count in 0..8, or stop');
  }
  const first = peers.length;
  const joins = await Promise.all(Array.from({ length: value.count - first }, (_, i) => connect(first + i)));
  console.log(JSON.stringify({ clients: peers.length, joins }));
}

const input = readline.createInterface({ input: process.stdin });
let commands = Promise.resolve();
input.on('line', line => { commands = commands.then(() => command(line)).catch(fail); });
input.on('close', () => { if (!closing) commands = commands.then(close).catch(fail); });
process.on('SIGTERM', () => { void close().then(() => process.exit(0)).catch(error => fail(error)); });
process.on('SIGINT', () => { void close().then(() => process.exit(130)).catch(error => fail(error)); });
