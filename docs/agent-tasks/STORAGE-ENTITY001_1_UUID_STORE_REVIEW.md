# STORAGE-ENTITY001.1 — Unabhängiger Implementierungsreview

Reviewer: `feature_catalog`, 10.10.2026. Ausschließlich dieser Report gehört
zum Mainauftrag; eigene Probeänderungen liegen in der isolierten Reviewkopie.

## Ergebnis

**CLEAN** für den ausdrücklich begrenzten fresh/strict Format5-Storageproducer.
Kein bestätigtes Finding und keine Runtimekorrektur angefordert. Originalsuite
und eigene Probes sind in Debug und Release bestanden, striktes AlltargetClippy
und Fmt ebenfalls. Alle 16 Freeze-Dateien sind nach dem Probe-Restore bytegleich
und Main wurde abschließend gegen dasselbe Manifest geprüft.

Der eigene Format5-Opener, getrennte UUIDschnittstelle, genaue Codecgrammatik,
gemeinsame Vorprüfung, atomare Mutation, lokale Encounteredprüfung und
vollständige Startvalidierung erfüllen den reservierten .1-Vertrag. Die
separate .2-Migration und alle Gameplayconsumer werden damit nicht abgenommen.

## Eingefrorene Basis und Dateibesitz

Root hat den endgültigen 16-Datei-Snapshot auf die isolierte Basis
`3a5b4d62866d5f81d9820ec05dd3843389d32d90` nach
`.codex-tmp/storage-entity001-review` kopiert und ausdrücklich zum Review
freigegeben. Nur 15 Storage-Dateien und der .1-Brief wurden übernommen; keine
bewegten World-/Registryquellen oder Lockfileänderungen. Target ausschließlich
`.codex-tmp/storage-entity001-review-target`, höchstens zwei Cargojobs.

Freeze: `.codex-tmp/storage-entity001-freeze.sha256`, Manifest-SHA256
`de74ec466245ccb965d8b31e5e72516a60624e1cd7a85f39038ef193c4a22c0d`.
Alle 16 Filehashes wurden vor Codeprüfung und nach der gesamten originalen
Debug-/Release-/Clippy-/Fmtabnahme erneut bestätigt. Der endgültige Briefhash
ist `65e7c90ad322be4fd8b1a5f6e763fc8c5c55c0001d9b7df92ed93e7cbef854cb`.

Designhash: `b4380b3699b34f8b7bd242e3afe79117523e8b3b847d6f835fe0e8cfae80ea83`;
Designreviewhash: `26792fbc4706720c25a9079957e6c56fd7fd435367b15d9d4e2cae4c36a6c072`.
R1–R4 und Roots zusätzliche Format5-spezifische PluginId-Admission von
4096 UTF-8-Bytes sind verbindlich. Alle .1-Migrationsforderungen beziehen sich
auf unverändernde Format3/4-Refusals; vollständige 4→5-Migration ist .2.

Der gesamte alte neun-Tabellen-Migrationsvalidator ist gegenüber der isolierten
Basis bytegleich, SHA256
`7c773d3cba1e1fe407ce1959781677b1306ec08208537ae24a37b592359eed12`.
`migration.rs`, `migration/validation.rs`, `migration/tests.rs` und Cargo.lock
haben keinen Gitdiff. Lockhash
`3b19aa0df48b04b1e6362c6ba06cb2c6e196232254fe0faf64b284c1a7c98b73`.

## Vertrags- und Quellenprüfung

Vollständig gelesen: finale neue UUID-API, Rawcodec, gemeinsame Engine,
Memory-/redb-UUIDproducer, kompletter zwölf-Tabellen-Validator und alle neuen
Unit-/Integrationstests; alle Änderungen in Error/Generation/Lib/Store sowie
beiden bestehenden Backends. Die vorhandenen Generationadmission-, Journal-/
Receiptcodec-/Replay-/Watermark- und Freshnesspfade wurden mitgeprüft. Lokale
AGENTS/CLAUDE, Storage-INVARIANTS/README, Architektur und ADR0004/0007 gelesen.

Originalpin: Minecraft 1.21.8, Protokoll772, Paper Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Alle zwölf Dateien aus
`.codex-tmp/storage-entity001-reference.sha256` wurden erneut gehasht; Manifest
`61d43cf3e4af7ba1bc1b451ba67943b17faa8ad3b8804d436a662d33ac7d0e89`.
Die bereits vollständig gelesenen Vanilla-/angewandten `EntityStorage`,
`PlayerDataStorage` samt vollständigem Sourcepatch, UUID-/Savepfade von
`Entity`, vollständige relevante `ItemEntity`-/Moonrise-Chunkentitysavepfade
und `Util.safeReplaceFile`/`safeReplaceOrMoveFile` samt Callees bleiben die
Referenz für die Trennung persistenter UUID, Wirehandle und Spielerdateien.

Paper speichert chunkweise Entityregionen und UUIDweise Spielerdateien auf
getrennten Pfaden. Daraus folgt kein Player↔Entity-Commit. FerrumCs Format,
Bounds, Epoch-/Cursor-, Corruption- und Retryregeln sind eigene Verträge.
Dies ist Quellenprüfung mit FerrumC-Vertragstests, keine Live-Paper-
Differenzialabnahme oder Vanilla-NBT-/Anvil-Interoperabilität.

## Response- und Consumer-Matrix

| Grenze | Prüfergebnis und konkrete Evidenz |
| --- | --- |
| API/Modi | Sechs dyn-kompatible Methoden, private Key-/Record-/Write-/Entry-/Epoch-/Page-/Cursorfelder, vollständige Identity und expliziter UUIDmode. Compilefail für Wire-/Player-ID, Seed-only und Cursorforge. Unsupported auch für leere nicht adoptierte Requests und alle vier alten Entitymethoden im UUIDmode. |
| Bytegrammatik | Key24, FCUE22, Locator20/12, Index32/empty, Epoch16/8; BE-, Nil-/MaxUUID-, Chunkorigin-/Offset- und malformed/trailing/oversizefälle geprüft. Opaque Body bleibt ≤256KiB und ohne Siminterpretation. |
| R1 online | Pointlookup prüft alle angetroffenen direkt adressierbaren Beziehungen im selben Snapshot. Foreign-Dimension-Locator wird tatsächlich auf Record/Index/Epoch/Descriptor geprüft. Gültiger anderer Scope ergibt lokales None/false; beschädigtes bekanntes Ziel verweigert. Unbekannter Indexorphan bleibt lokales None/false, aber wird bei Zielsave/Page/strict Reopen verweigert, ohne Repair. |
| Vollvalidator/R4 | Exact zwölf Tabellen/Typen/Metadaten, alle Rows, beide Indexrichtungen, wirkliche Generatoridentitäten, Cap4096, unscoped Player/Journal/auch Emptyreceipt, Kontiguität ab0, Watermark, Receiptgrammatik/-range/-nonoverlap. Keine Altdeclaration, Flatbaseline oder Seed0 wird erfunden. Altmigrationsvalidator bytegleich. |
| Save/Delete/Move/R3 | Save wechselt Chunk innerhalb Dimension; Move gleiche World/UUID mit verschiedenen Dimensionen, vorhandener Source und freiem Ziel. Samekey/Crossworld/andereUUID und doppelte scopedKey/WorldUUID verweigern. Gleiche UUID in anderer Welt unabhängig. 64 Writes/4MiB inkl Key+Envelope. Jede distinct geänderte Chunkepoche genau+1, maximal128 Keys. |
| Atomizität | Memory besitzt gemeinsamen UUIDstatelock innerhalb Generationmutationsgrenze; vollständige Vorprüfung vor synchronem Apply, keine falliblen Returned-error-Schritte danach und kein Guard über Await. redb führt Encoding/Vorprüfung/Rows/Commit in einer echten WriteTransaction auf spawn_blocking aus. Eigene39 tatsächliche Schreibcutpoints über sechs Mutationstypen erhalten alle zwölf Rawtabellen und strikte Validation. |
| Epoch/No-op | Exakte konsistente Raw-no-ops bei MAX erlaubt; reale Änderung/Deletion/Move/MAXbatch verweigern atomar. Fehlender Index wird durch identischen Save nicht repariert. Historische leere Epochen bleiben nach Reopen und verweigern neue Freshbaseline. No-op und Fehlerbatch erhalten alte Cursor; fremde Chunks invalidieren sie nicht. Schemaänderung desselben Chunks invalidiert sie. |
| Pages/R2 | Limit1..64, ≤1MiB exakt24+22+Payload, höchstensLimit+1 Indexrows/ein borrowed Lookahead. Erste Range Included(raw16prefix), Folge exklusiv lastUUID, exakter Prefix mit all-FF-Ende ohne Wrap. Nil-/MaxUUID, Count-/Bytecaps und Fortschritt/Ende geprüft; Payloadkopie erst nach bounded Relationadmission. |
| Cursor | Weak Backendincarnation, vollständiger Chunk, Identity und historischer Epoch. Exakte Backend/Chunk/GenerationIdentity-Refusal bereits vor Descriptor-/Rowlookup; Epoch vor Indexscan. Gleiche Arc-authority akzeptiert, foreign/reopen verweigert. Cursor erhält Backend/DB nicht am Leben. |
| Parallelität | Eigene gleichzeitig gestartete gegenläufige Dimensionmoves bleiben vollständig konsistent; zwei Moves derselben Source-WorldUUID haben genau einen Gewinner und exakt einen lebenden scoped Record. Memory und redb gleich; abschließender strict Reopen akzeptiert. Keine Konsistenz über mehrere separate Reads behauptet. |
| Producer→Startup | Eigene erfolgreiche Chunk-/Overlay-/maximalPlayer-/Plugin-/Journal-/Receipt-/UUIDwrites werden gemeinsam durch strict Reopen akzeptiert. Auch 4096-Byte-EmojiPlugin, 512-Byte-Key, 1MiB-Value; Journalreplay nach Reopen bleibt normalisiert und Konflikt verweigert. |
| Pluginpolicy | Put/Get/Delete/Keys begrenzen Format5-PluginId vor Keyallocation/IO auf4096 UTF-8-Bytes. ASCII/Multibyte4096/4097, Fehler-No-publish, Memory/redb/Restart und unveränderte Legacy-/Metadatamodi getestet. Keine generische PluginIdcap behauptet. |
| Ressourcen | Haupt-redb-Cache64MiB; privater Disk-Receiptindex16MiB, owned Fileidentity mit Cleanup nach Handlefreigabe und Erhalt ersetzter Pfade. Streaming Rows/Pointlookups, keine Ganzweltcollection. Bounded Input/Plan/Page und decodierte Row, redb-/Objektoverhead zusätzlich; Memory hält gesamten Datenbestand. Keine RSS/TPS-Garantie aus encoded Caps. |
| Neustart/Retry | Eigener Aftercommit-Lostreplytest zeigt vollständig erhaltene Movebytes nach strict Reopen. Move-Retry verweigert fehlende Source und verändert nichts. Dies beweist keine allgemeine Retry-/CAS-/Ownership-/Receiptsemantik. Format3/4-Refusals erhalten Rawrows/Catalog, keine physische Sourcehashgarantie aus redb Open/Close. |

## Unabhängig ausgeführte Abnahme

Originalsnapshot, jeweils `--offline --locked -j2`:

- Debug: **212 bestanden**, 0 Fehler/ignoriert; 167 Unit +38 Integration +7
  Doc-/Compilefailtests. Log `.codex-tmp/storage-entity001-review-debug.log`.
- Release: **212 bestanden**, 0 Fehler/ignoriert; gleiche Matrix.
  Log `.codex-tmp/storage-entity001-review-release.log`.
- Strict AlltargetClippy: grün, `-p ferrumc-storage --all-targets -- -D warnings`.
  Log `.codex-tmp/storage-entity001-review-clippy.log`.
- `cargo fmt -p ferrumc-storage --check`: grün, leerer Fehlerlog
  `.codex-tmp/storage-entity001-review-fmt.log`.

Zusätzlich neun eigene Probes, keine aus Main übernommenen Runtimekorrekturen:
vier Publicintegration-, drei Engine- und zwei private redb-Probes:

- Debug **9/9 grün**, Log `.codex-tmp/storage-entity001-review-probes-debug.log`.
- Release **9/9 grün**, Log `.codex-tmp/storage-entity001-review-probes-release.log`.
- Strict AlltargetClippy grün, Log
  `.codex-tmp/storage-entity001-review-probes-clippy.log`.
- Fmt grün, Log `.codex-tmp/storage-entity001-review-probes-fmt.log`.

Probequellen liegen ausschließlich unter
`.codex-tmp/storage-entity001-review/.codex-review-probes/`:

| Datei | SHA256 der endgültig geprüften Probe |
| --- | --- |
| `public.rs` | `4bd7c700ac04d7e74367f4564bdcdd15c164206ab4ce643239f232bf197737ec` |
| `private.rs` | `552d68617e38fa2c038592fb361391c20103b86d1b65bbf0efa6053667e37833` |
| `redb.rs` | `57de7df577d1c61abb4c15dcbec487ef23a3fbe838502f21b6d9fd5bc617be16` |

Für diese Probeabnahme wurden sie als `tests/reviewer_uuid_contracts.rs`,
`src/reviewer_probes.rs`, `src/redb_store/reviewer_probes.rs` installiert; die
letzten beiden über ausschließlich test-only `mod reviewer_probes` in Lib
beziehungsweise redb_store. Diese Includes wurden erst nach Originalabnahme
hinzugefügt, danach aus den vorab gespeicherten Originalbytes wiederhergestellt;
die drei installierten Probe-Dateien sind aus der Storagecrate entfernt.
Alle16 Freezehashes sind erneut grün, Log
`.codex-tmp/storage-entity001-review-final-hashes.log`; Altvalidation und Lockfile
weiter bytegleich. Keine Produktionsdatei in Main geändert, keine zusätzlichen
Dependencies oder Gitaktionen. Probes sind keine Produktions-API/Simaktivierung.

## Verbleibende Consumergrenzen

Weiter offen: STORAGE-ENTITY001.2 mit expliziter 4→5-Migration und checked
Backup-/Retryevidenz; Simcodec für Type/UUID/Position/Scope/Capacity und vollständige
Admission vor Publish; gemeinsame WorldStore/EntityStore-Backendauthority;
Chunkloadreservations/Dirtyacks, autosave/unload/restart und Gameplayaktivierung.
DUR-03/04/05 benötigen versionsgeführte Writer, Tombstones/CAS und langlebige
Receipts für Entity/Player/Containerownership mit Ack/Disconnect/Lostreplyregeln.

Kein Test dieses Producers übernimmt Javaevents, Itemphysics/Timers/Pickup/Merge,
Mobs/Survival oder deren Appconsumer. Auch OOMrollback, Processkill-/Powerloss,
Item-Conservation, Performanceparität und vollständiges Drop-in-Minecraft sind
hier keine abgenommenen Eigenschaften.
