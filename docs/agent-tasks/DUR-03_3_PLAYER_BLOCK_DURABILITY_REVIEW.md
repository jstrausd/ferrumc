# DUR-03.3 — unabhängige Vertragsabnahme

Stand: 11.10.2026. **CLEAN_CONTRACT; Storage-Implementierung freigegeben.**
Dies ist eine Designabnahme. Storage-Laufzeit, App-Aktivierung, Survival und
Paper-Gleichheit sind damit nicht abgenommen.

Root hat den vollständigen konkreten
[Vertrag](DUR-03_3_PLAYER_BLOCK_DURABILITY.md), die vollständige SOURCE_REVIEW,
die tatsächlichen Format6-Writer und die Full-/Overlay-/Blockentity-Komposition
geprüft. Die unabhängige Nachprüfung liegt unter
`.codex-tmp/dur03_3-root-review/assessment.json` und `verify_contract.py`.

| Festgeschriebene Grundlage | SHA-256 |
| --- | --- |
| DUR-03.3-Vertrag | `2edfc588fd8d7e96753f6beb4bb7c2ad50a064e4c266681ab5ff94550c4342ab` |
| DESIGN_EVIDENCE_FREEZE.json | `3710c57265084f9a6cd59d48fc77b9340fcd255bd0f235545c7ca639b92de92f` |
| echtes d3fca66c-Archivmanifest | `c148936b29cc58d7ac25b2904618523c3b3a163476a2bf380b1d00fadf07c7b0` |

Root hat alle 1224 Archivdateien, 461 ausgewählten Dependency-Dateien,
45 redb-Dateien, 82 Paper-Dateien und 16 Designartefakte frisch gegen ihre
ganzen Bytes und Bestände geprüft: keine Abweichung. 216 eigene
Grammatik-Arithmetikfälle bestätigen FCBQ, FCOB, FCBL und FCBR. Diese Fälle
führen keinen Runtimeparser oder Datenbankcommit aus.

Die freigegebene Umsetzung bleibt ausschließlich in `ferrumc-storage`, ohne
neue Dependencies. Format7 bindet die vollständigen ursprünglichen und neuen
Player-/Chunk-/Overlay-/Blockentity-Bilder an tatsächlich ausgegebene
Snapshot-Autorität. Alle Blockwriter verwenden dieselbe Chunkrevision;
Player-/Entity-CAS und gemeinsame Blockoperationen teilen Playerrevision,
Receipt-ID-Namespace, Kapazitäten und Workeradmission. Format6 und seine
FCTR-Grammatik bleiben erhalten. Unsichere Commit-/Antwortpfade behalten die
ursprüngliche Operation, bis ein tatsächlicher Receipt oder exakter Retry sie
auflöst. Eine fehlende Antwort erlaubt keine sichtbare Bestätigung.

Der Implementierungsagent besitzt nur `server/crates/ferrumc-storage/**`,
`.codex-tmp/dur03_3-runtime/**` und einen neuen RuntimeDelivery-Bericht. Der
Designfreeze und seine Artefakte bleiben unverändert. Laufende World-/Sim-WIP
wird nicht als akzeptierte Dependency übernommen. Finale Builds erfolgen auf
einer eigenen Kopie des echten d3fca66c-Archivs mit vollständigem Storage-Overlay
und unveränderten Außendateien. Root prüft anschließend unabhängig.

Die gesamte im Vertrag definierte tatsächliche Memory-/redb-/Fehler-/Race-/
Replay-/Malformed-/Budget-Matrix ist erforderlich, einschließlich einer
separaten Public-API-Fixture und unabhängiger tatsächlicher Tabellenprüfung.
Whole-Storage Debug/Release, striktes Clippy, fmt und rustdoc müssen bestehen.
Crash durch Prozessabbruch, Stromausfall, Live-Paper- und Clientvergleiche
bleiben gesonderte Abnahmen; Quellenlektüre ersetzt sie nicht.

Root hält **DUR-03.3a, explizite Offline-Migration Format6→7, vor DUR-05** fest.
Keine automatische Migration oder Live-Aktivierung ist Teil dieser Freigabe.
Downstream Sim/App erhält danach eigene Tickets für echte ausgegebene
Autorität und vollständige noch offene Gameplay-Abhängigkeiten.
