import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { createHash } from 'node:crypto'
import { createRequire } from 'node:module'

const [rootArgument, paperArgument, outputArgument] = process.argv.slice(2)
if (!rootArgument || !paperArgument || !outputArgument) throw new Error('usage: export_prismarine.mjs ROOT PAPER_TSV OUTPUT_DIR')
const root = path.resolve(rootArgument)
const paperPath = path.resolve(paperArgument)
const out = path.resolve(outputArgument)
if (fs.existsSync(out)) throw new Error('use a fresh output directory')
const require = createRequire(path.join(root, 'tests/blackbox/package.json'))
const mc = require('minecraft-protocol')
const data = require('minecraft-data')('1.21.8')
const sha = data => createHash('sha256').update(data).digest('hex')
const schemaPath = path.join(root, 'fixtures/protocol/1_21_8/protocol.json')
const schema = JSON.parse(fs.readFileSync(schemaPath, 'utf8'))
assert.deepStrictEqual(data.protocol, schema, 'installed protocol schema must match the vendored authority exactly')
assert.equal(require('minecraft-protocol/package.json').version, '1.66.2')
assert.equal(require('minecraft-data/package.json').version, '3.111.0')
assert.equal(data.version.version, 772)
const serializer = mc.createSerializer({ state: 'play', isServer: true, version: '1.21.8' })
const decoder = mc.createDeserializer({ state: 'play', isServer: false, version: '1.21.8', noErrorLogging: true })
const text = fs.readFileSync(paperPath, 'utf8')
const rows = text.trimEnd().split('\n')
assert.equal(rows.shift(), 'case\tkind\tentity_id\tv1\tv2\tv3\tmetadata_hex\tpacket_hex')
const output = ['case\tkind\tentity_id\tv1\tv2\tv3\tmetadata_hex\tpacket_hex']
const semantics = []
const item = (itemCount, components = [], removeComponents = []) => itemCount === 0
  ? { itemCount: 0 }
  : { itemCount, itemId: data.itemsByName.stone.id, addedComponentCount: components.length, removedComponentCount: removeComponents.length, components, removeComponents }
const items = {
  metadata_item_empty: item(0),
  metadata_item_stone_1: item(1),
  metadata_item_stone_64: item(64),
  metadata_item_components: item(3, [
    { type: 'custom_data', data: { type: 'compound', value: { bytes: { type: 'byteArray', value: [0, -1, 1, -1] } } } },
    { type: 'custom_name', data: { type: 'string', value: 'wire ä' } },
    { type: 'repair_cost', data: 128 }
  ]),
  metadata_item_removed: item(2, [], [{ type: 'max_stack_size' }])
}
for (const row of rows) {
  const [name, kind, entityText, xText, yText, zText, metadataHex, packetHex] = row.split('\t')
  const entityId = Number(entityText)
  const x = Number(xText); const y = Number(yText); const z = Number(zText)
  let packet
  if (kind === 'motion') packet = { name: 'entity_velocity', params: { entityId, velocity: { x, y, z } } }
  else if (kind === 'take') packet = { name: 'collect', params: { collectedEntityId: entityId, collectorEntityId: x, pickupItemCount: y } }
  else if (kind === 'metadata') {
    let metadata
    if (name.startsWith('metadata_empty_')) metadata = []
    else if (name === 'metadata_primitives') metadata = [
      { key: 0, type: 'byte', value: -1 }, { key: 1, type: 'int', value: -2147483648 },
      { key: 2, type: 'float', value: -1.25 }, { key: 3, type: 'string', value: 'stone ä' }
    ]
    else {
      if (!(name in items)) throw new Error(`unknown independently declared item case: ${name}`)
      metadata = [{ key: 8, type: 'item_stack', value: items[name] }]
    }
    packet = { name: 'entity_metadata', params: { entityId, metadata } }
  } else throw new Error(`unknown packet kind: ${kind}`)
  const encoded = serializer.createPacketBuffer(packet)
  assert.equal(encoded.toString('hex'), packetHex, `independent constructed packet: ${name}`)
  const original = Buffer.from(packetHex, 'hex')
  const decoded = decoder.parsePacketBuffer(original)
  assert.equal(decoded.metadata.size, original.length, `independent decoder consumed complete packet: ${name}`)
  assert.deepStrictEqual(decoded.data, packet, `independent semantic decode: ${name}`)
  semantics.push({ case: name, packet })
  output.push([name, kind, entityText, xText, yText, zText, metadataHex, encoded.toString('hex')].join('\t'))
}
assert.equal(rows.length, 30)
fs.mkdirSync(out, { recursive: true })
fs.writeFileSync(path.join(out, 'prismarine.tsv'), output.join('\n') + '\n')
fs.writeFileSync(path.join(out, 'semantics.json'), JSON.stringify(semantics, null, 2) + '\n')
const files = [schemaPath, paperPath, path.join(root, 'tests/blackbox/package.json'), path.join(root, 'tests/blackbox/pnpm-lock.yaml'),
  require.resolve('minecraft-protocol'), require.resolve('minecraft-data'), require.resolve('minecraft-protocol/src/transforms/serializer'),
  require.resolve('minecraft-protocol/src/datatypes/compiler-minecraft')]
const provenance = { status: 'passed', protocol: 772, version: '1.21.8', rows: rows.length, node: process.version,
  packages: { 'minecraft-protocol': '1.66.2', 'minecraft-data': '3.111.0' },
  scope: 'independent compiler encoding of declared semantic packet inputs and semantic decoding of actual Paper bytes; no gameplay acceptance',
  inputs: Object.fromEntries(files.map(file => [file, sha(fs.readFileSync(file))])),
  helper_sha256: sha(fs.readFileSync(new URL(import.meta.url))),
  outputs: Object.fromEntries(['prismarine.tsv', 'semantics.json'].map(file => [file, sha(fs.readFileSync(path.join(out, file)))])) }
fs.writeFileSync(path.join(out, 'provenance.json'), JSON.stringify(provenance, null, 2) + '\n')
process.stdout.write(JSON.stringify({ rows: rows.length, packet_sha256: provenance.outputs['prismarine.tsv'] }) + '\n')
