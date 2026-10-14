# Survival-Laufzeitkorrekturen vom 14.10.2026

Die finale Release-Binary korrigiert die belegte Entity-/Inventar-Paketflut,
Werkzeug- und Pflanzenabbau, Wasseraktivierung und die Vitals-Erfassung beim
Verbindungsabbau. Vier private Protokoll-772-Serverproben sowie die finalen
Integrations- und Speicherprüfungen sind bestanden. Vollständiges Minecraft/Paper
und sämtliche Survival-Etappen bleiben offen; die nächste Reihenfolge steht im
[Portierungsplan](SURVIVAL_PORT_NEXT_STEPS.md).

Basis ist `83e681a61f566a24098d641ed8e854d32777893a`. Die getestete Binary hat
SHA-256 `8853707b02968a015821b841f6ef4a4dc67e0186d96329f765753c0a91bc57e6`
und 16.170.416 Bytes. Sie wurde aus dem isolierten Worktree mit
`cargo build --manifest-path server/Cargo.toml --release --locked --offline --jobs 2 -p ferrumc-app`
gebaut. Fremde Registry-/World-Arbeiten wurden weder importiert noch gebaut.
Persönliche Server, Konfigurationen und Welten wurden nicht für Versuche verwendet.

## Was sich ändert

| Consumer | Resultierendes Verhalten |
| --- | --- |
| Session und Entity-Views | Die bestehende 256-Slot-FIFO reserviert 64 Slots für Pflichtverkehr. Bewegungen werden pro Viewer zusammengefasst und unveränderte Posen/Metadaten unterdrückt. Spawn und Metadaten werden gemeinsam zugelassen, höchstens 16 neue Entities pro View/Viewer/Tick; volle gemeinsame Reserve verschiebt beide. Echte bekannte Metadata-/Remove-/Pickup-Refusals bleiben Fehler. |
| Pickup in App | Auch ein volles oder für Food gepinntes Inventar bestätigt die komplette Restmenge beim echten Item-Owner. Ohne übertragene Menge entfallen unveränderte Inventar- und Equipment-Pakete. Erfolgreiche und teilweise Pickups werden weiterhin synchronisiert. |
| Mining | Aktuelle Bodenflags und tatsächliches Wasser an der stehenden Augenposition ändern die Geschwindigkeit ohne Verlust laufender Arbeit oder des genauen Werkzeugzustands. Härte null ist sofortig, einschließlich airborne/submerged; besondere Scheren-/Schwertregeln sind berücksichtigt. Eine nächste Aktion wartet begrenzt auf echten vorherigen Verschleiß und erfasst dann den neuen Stack. |
| Pflanzen-Loot | Gras/Farn ohne Schere hat die originale 1/8-Samenbedingung, keinen garantierten Grasdrop. Scheren liefern die kleinen Pflanzen. Gültige hohe Pflanzen entfernen beide Hälften; vollständige Drop-/Water-Zulassung geschieht vor dem ersten Write, Refusal erhält Welt und RNG. |
| Wasser | Aktivierung entdeckt vorhandenes Wasser sowie neu resident gewordene Spalten und deren residente Kardinalnachbarn. Fortsetzbare begrenzte Scans behalten ihren Cursor bei Queue-Druck; sie laden keine zusätzlichen Spalten. Eimer, Quellenentnahme und normale Platzierungen verwenden den echten Owner. |
| Leave-Save | Ein vollständiger tatsächlicher Vitals-Snapshot wird unmittelbar vor Actorentfernung veröffentlicht. Die App behält ausschließlich echte Beobachtungen bis zum bestätigten Leave; aktuelle Ownerdaten haben Vorrang. Damit bleiben auch stille Exhaustion-/Saturationänderungen verfügbar. Die 1024 Slots beginnen mit None und eine neue Verbindung erhält keinen alten Snapshot. |
| Diagnostik | Mandatory-Refusals nennen Producer, Paket-ID und Belegung der Session-FIFO. Diese ist eine andere Warteschlange als die im alten Log leere Writer-Queue. |

Die alten `UnknownPlayer`-Zeilen belegen eine entfernte Session, während der
Prozess weiter tickte. Ohne die erste Ablehnung aus diesem alten Lauf ist sein
ursprünglicher Producer nicht eindeutig bestimmt. Die korrigierten Druckquellen
wurden unabhängig in echten Paketaufzeichnungen nachgewiesen.

## Private Versuche mit der finalen Binary

| Versuch | Tatsächliches Ergebnis |
| --- | --- |
| Entity-Druck und Gehen, Port 25683 | 20 echte Item-Entities und mehrere Dutzend Mobs; alle 24 expliziten Pig-Anfragen vom Owner angenommen. Vier Gehstrecken x=8,5↔64,5 ohne Abbruch. Drei Sekunden Idle: null Item-Motion, null identische Item-Metadata, null unveränderte Window-Items. Die bewegten Mobs erzeugten 2003 tatsächliche Motion-Pakete. Alle 20 Items nach Wiederbeitritt und redb-Neustart erhalten; beide Serverstopps Exit 0. |
| Mining, Port 25694 | Frühes FINISH bei Stein: Hand 7472 ms, Holz 1142 ms, Eisen 404 ms, Gold 168 ms. Ground-only und Wet→Dry erhalten Arbeit und genau einen Verschleiß. Airborne-Gras mit Schere: 72 ms, ein tatsächlicher Drop/Pickup. Zwei schnelle Pflanzenaktionen: zwei zusätzliche Pflanzen und zwei Verschleißquittungen. 128 Hand-Grasblöcke liefern 25 Samen und kein Gras; alle Samen nach echtem Neustart erhalten. Beide Stopps Exit 0. |
| Wasser, Port 25721 | Vollständiger Radius-7-Diamant mit 113 Zellen, reale Grenze x=15→16 und Level-7-Endzelle. Survival-Wassereimer wird beim Gießen zum Eimer, über Neustart erhalten und beim Aufnehmen wieder Wassereimer. Nach Quellenentnahme alle 113 Zellen Luft; beide Stopps Exit 0. |
| Natürliche Welt, Port 25722 | Vier zuvor ungesehene Chunkzentren mit echten kompletten Chunkdaten, Bäumen, Blättern, Gräsern und Erzen. View2, Creative-Flugprobe ohne grafische Clientphysik. Wiederbeitritt am letzten Zentrum und sauberer Stopp Exit 0. Zeiten und die ausdrücklich begrenzte Aussage folgen unten. |

Im natürlichen Versuch kamen die neuen Zentren nach **1674 / 1209 / 584 / 547 ms**;
die jeweils vollständige 25-Spalten-Sicht nach **6380 / 3315 / 3037 / 3194 ms**.
Das sind vier einzelne Proben mit Sprüngen um drei Chunks, keine allgemeine
Durchsatzmessung oder Survival-Bewegungsabnahme. Drei Zentren enthielten Bäume
und Gras; ein wasserreiches Zentrum enthielt beides nicht. Alle vier enthielten
Erze. Der Generator bleibt bei kalter Nachfrage spürbar verzögert. Als Nächstes
müssen die echte Generation und der Vorlauf bei gewöhnlicher Gehgeschwindigkeit
profiliert und verbessert werden; diese Korrektur behauptet keine abgeschlossene
Vanilla-Weltgenerierung oder garantierte Chunk-Latenz.

Zusätzlich wurden vor Regressionen tatsächliche Sim/Session-Experimente für
beide Pflanzenhälften, erzwungene kleine Session-FIFOs und stilles Sprinten
mit Leave im selben Owner-Tick ausgeführt. Die Driver-Regressionsprüfung liest
die danach erhaltenen neun Felder und speichert aktuelle Inventorydaten mit dem
echten PlayerStore/Schema5. Sie ersetzt keine vollständige erzwungene
Connection-Teardown-Abnahme; reguläre Connection-Saves sind in den echten
Wiederbeitritt-/Neustartproben erfasst.

## Finale Prüfungen und Nachweise

Implementierung, private erste Laufversuche und deren Korrekturen erfolgten vor
den neuen Regressionen. Danach wurde der integrierte Stand geprüft und genau
seine finale Release-Binary erneut privat ausgeführt.

| Prüfung | Ergebnis |
| --- | --- |
| `cargo test -p ferrumc-sim` | 543 bestanden, inklusive Integrations-/Rustdocprüfungen |
| `cargo test -p ferrumc-session` | 250 bestanden, inklusive Integrations-/Rustdocprüfungen |
| `cargo test -p ferrumc-app` | 597 bestanden, eine explizite Java-Pluginprüfung ignoriert |
| `cargo fmt --all --check` | Exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0 |
| Rustdoc der drei betroffenen Crates, `RUSTDOCFLAGS=-Dwarnings`, ohne Dependencies | Exit 0 |
| Finaler Release-Build | Exit 0; 25 erfasste Build-Sourcepfade vor/nach dem Build bytegleich |

Die ignorierte Prüfung benötigt eine ausdrücklich konfigurierte JVM und die
Maven-gebaute Bridge sowie gewöhnliche Plugin-JARs. Daraus wird keine
Java-/Paper-Plugin-Abnahme abgeleitet. Alle Cargo-Prüfungen verwenden den
privaten Targetpfad, locked/offline und zwei Jobs.

Vollständige Logs, Paketaufzeichnungen, alte Fehlversuche, Reference-Manifeste,
Buildsource-Archiv und Binary liegen unter
`.codex-tmp/survival-port-20261014/`. Finale Ergebnisse:

- `session/entity-walk-final-1/result.json`, SHA256 `42f00248434f6b23b65d3419837a116cc7689f1d5e0912af975ffedef5c42a40`.
- `mining/mining-try-final-1/result.json`, SHA256 `86e9d5b2ba54e44cbb944e868227504757f1c20837244d8cb96e8053eb79bee1`.
- `water/water-final-1/result.json`, SHA256 `a57896f1ea947fd8e8903e76b08d7daa41d11c71e5ea8bbe4f3a65829cd2a308`.
- `water/natural-streaming/natural-streaming-final-2/result.json`, SHA256 `e25e72f73cc1cd13b38b5fa2ce1373c4c947e578aab3c26ee09ff6f511198988`.
- `final-release/source-manifest.json`, SHA256 `fb5a6b3b630f5a0321bbeaaa9e446fb4b768added1daeb21b69bcbfb7ed196e4`.

## Originalreferenz und verbleibende Grenzen

Referenz ist Paper 1.21.8 Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`, mit vollständig angewandten
Originalquellen sowie Source-/Feature-Patches. Die drei Agentberichte enthalten
gehashte Vollquellen und Methodenmatrizen. Root las zusätzlich die vollständigen
PlayerList-Save-/Remove- und PacketListener-Disconnectpfade samt relevanten
Patches und dem Incremental-Saving-Feature. Siehe [Referenzvertrag](PAPER_REFERENCE.md).

FerrumCs feste FIFO-Reserve, Entity-Pacing und native Vitals-Timer sind eigene
begrenzte Verträge. Eine Quellenprüfung oder FerrumC-Probe ist kein bestandener
Paper-Differentiallauf. Die per-Viewer-Pose bezeugt FIFO-Aufnahme; spätere lossy
Writer-Drops sind kein End-to-End-Ack. Periodische Tracker-Heilung, originale
relative Bewegung/Velocity/Passenger- und Java-Eventsemantik bleiben offen.
Mining fehlen unter anderem vollständige Komponenten, Enchants, Effekte,
Loot-/XP-Kontexte; der native Seed-RNG ist nicht der persistierte Vanilla-RNG.
Wasseraktivierung ersetzt weder Lava/Mischungen noch vollständige Schwimm-,
Strömungs- oder Ertrinkensconsumer. Player↔Entity-Crashatomik und der komplette
Featurekatalog bleiben weitere Portierungsarbeit.
