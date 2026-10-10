# FerrumC v2 — Prüfbericht des Hosting-Forks

Stand: 10.10.2026. Fork: [jstrausd/ferrumc](https://github.com/jstrausd/ferrumc).
Ausgangspunkt: upstream `rework/v2-skeleton`, Commit
`8315da2c407a66ef591d2ef9de158aa74e7f6bc4`.
Prüfumgebung: Linux x86_64, Rust/Cargo 1.99.0, Java 21, Maven 3.9.11.

`main` enthält v2 und ist der einzige Branch im Fork. Der erste Hosting-PR
wurde integriert; vor dem Entfernen der übrigen 19 Fork-Branches wurde ein
vollständiges Git-Bundle samt Branchliste außerhalb des Checkouts gesichert.
Die Upstream-Branches wurden nicht verändert.

Die lokale Vollabnahme der N02/I03/S02-Erweiterung verwendet den
Produktionscode bis `a213a491`; spätere Dokumentations-/Harness-Commits ändern
diesen Servercode nicht. Aktuelle Code- und Client-Ergebnisse stehen unten.
Die getrennt eingefrorene Release-Ressourcenmessung verwendet weiterhin
`7333f404`; sie misst die neueren Funktionen nicht. GitHub-CI und lokale
Containerartefakte werden separat ausgewiesen.

## Implementierter Stand

- Play-Keep-alive mit exakten IDs, absoluten Fristen, fairer Bearbeitung und
  cancellation-sicherer Socket-/Chunk-Übergabe.
- Absolute Pre-Play-Phasenfristen und eine First-Byte-Teilframefrist von
  höchstens zehn Sekunden. Ignorierte vollständige Pakete und Byte-Drips
  verlängern sie nicht; gepipelinete Reste behalten ihren Empfangszeitpunkt.
- Serverautoritative normale Links-/Rechtsklicks im Spielerfenster 0 erhalten
  Komponenten bei Split/Merge/Swap und synchronisieren Slots/Cursor zurück.
  Spectator verändert weder Spieler- noch Truhenslots. Rezepte, Armor-Inserts
  und weitere Klickarten bleiben offen.
- Truhen-Klicks prüfen Fenster-/State-ID; der Driver vergleicht die letzte
  Chest-Sicht atomar. Veraltete Klicks verändern keine Items. Ein einziges
  spielereigenes Cursor-Item bleibt bei voller Tasche über Fensterwechsel,
  Rejoin und erfolgreiche Saves erhalten. Neue Saves verwenden Schema 3 für
  vollständige unterstützte Komponenten; JSON 1/2 migrieren ausdrücklich.
  Eine begrenzte UUID-Reserve verhindert
  paralleles Laden bis nach Save und bestätigtem Leave. Unklare Freigaben
  oder fehlgeschlagene Saves sperren die UUID bis zum Instanzneustart.
- Der Driver bleibt während Connection-Cleanup aktiv. Expliziter Stop und
  geschlossener Commandkanal durchlaufen denselben finalen Storage-Flush.
  Checked-Capture erfasst höchstens vier Overlays je Übergabe; Release-Barrieren
  warten auf jeden Teilbatch vor Ticketfreigabe, Shutdown drainiert den Rest.
- Rust 1.99.0 numerisch gepinnt. Bestehende Compiler-/Clippy-Diagnosen wurden
  crateweise korrigiert und einzeln committed. Die strenge Workspace-Prüfung
  besteht auch für die N02/I03/S02-Erweiterung.
- Optionale Java-21-Brücke gegen die echte, timestamp-gepinnte Paper-1.21.8-API:
  gewöhnlicher `plugin.yml`-Loader, Lifecycle/Konfiguration, deklarierte Commands,
  Aliases/Namespaces, aktuelle Spieleridentität/-nachrichten und Join-/Quit-Events.
  Argument-Suggestions verwenden den echten Bukkit-TabCompleter-/Executor-Pfad,
  mit originaler Alias-Schreibweise, leeren Argumenten, UTF-16-Bereichen und
  begrenzten Antworten. Stateless Completion-Waits beachten Play-Frist und Stop.
- Docker/Pterodactyl enthalten Java-Laufzeit, Adapter, Lizenzhinweise und
  entsprechende Quellen. CI verwendet `main` und verpflichtende Java- und
  unabhängige Client-Gates.

FerrumC bleibt der Rust-Spielserver; seine Welt und Netzwerkverbindungen
gehören der Rust-Laufzeit. Die Java-Brücke implementiert eine getestete
API-Teilmenge. Die Grenzen stehen im
[Adaptervertrag](../server/java-bridge/README.md) und
[ADR-0009](adr/0009-java-plugin-compatibility.md).
Der neue PlayDisconnect mit begrenztem NBT-Grund wurde über
`packets.toml` und den Generator erzeugt; Critical/Mandatory priorisiert ihn
vor bereits wartendem Weltverkehr. Unabhängige Wire-Fixtures prüfen das Paket.

## Code-Prüfung

Die ergänzende [Paper-Quellenprüfung](PAPER_REFERENCE.md) vom 10.10.2026
verwendet den offiziell zugeordneten Paper-1.21.8-Build-60-Commit und das
bereits gepinnte API-Sources-JAR. Gelesen wurden Serverpatches für Items und
Container sowie CraftBukkit-Scheduler-/Lifecycle-Quellen. Vollständige
Vanilla-Basisquellen und Vergleichsläufe gegen einen laufenden Paper-Server
sind noch offen. Diese reine Referenzprüfung ergänzt die untenstehenden
FerrumC-Tests und belegt keine weitere Gameplay- oder Plugin-Kompatibilität.

Aktuelle lokale Vollabnahme der N02/I03/S02-Erweiterung; CI führt zusätzlich
`cargo nextest run --workspace --all-targets --locked` aus. Ein erfolgreicher
lokaler Cargo-Testlauf ist kein vorweggenommenes GitHub-Ergebnis.

| Prüfung | Ergebnis |
| --- | --- |
| `cargo fmt --all --check`, `git diff --check` | Bestanden. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Bestanden, ohne CLI-Lint-Ausschlüsse. Drei dokumentierte trait-lokale Ausnahmen betreffen den von `async_trait` erzeugten `double_must_use`-Lint. |
| `cargo test --workspace --locked` | 2.102 Tests in 150 Gruppen bestanden, einschließlich Doc-Tests und nativer Plugin-Fixtures. Ein expliziter Java-Artefakt-Test ist standardmäßig ignoriert und wurde separat erfolgreich aufgerufen. |
| App-Unit-Abnahme | 258 Tests bestanden; Pre-Play-/Play-Fristen, Inventar, NBT-Floatbits und Speicherfehlerpfade enthalten. |
| Rust/JAR-Socket-Abnahme | `cargo test -p ferrumc-app --locked --test java_compat -- --ignored` bestanden; fehlende Java-/JAR-Artefakte führen bei Aufruf zum Fehler. Dieser Aufruf ist obligatorisch in CI. |
| `cargo xtask generate --check` | Bestanden; eingecheckter Protokollcode aktuell. |
| `cargo metadata --no-deps --locked --format-version 1` | Bestanden. |
| Java `mvn --batch-mode --no-transfer-progress verify` | 11 Parser-/Archiv-Unit-Tests und sieben echte Prozessintegrationstests bestanden; keine übersprungenen Tests. |

Die Java-/Rust-Brücke wurde unabhängig auf Prozessbesitz, Reaping, begrenzte
Frames/Queues/Metadaten, IDs, absolute Fristen inklusive Warteschlange,
Cancellation, Startup-Rollback und Empfängergrenzen geprüft. Keine offenen
Blocking-Findings im implementierten Subset. Commandkonflikte werden auch
bei unterschiedlicher Groß-/Kleinschreibung vor Client-Annahme abgewiesen.

Sieben dauerhafte Performance-Harness-Regressionstests prüfen mit Docker-Attrappen
Ownership, Log-/Start-/Extraktionsfehler und Cleanup; alle bestanden und sind
verpflichtend in CI. Sie starten keine Container oder Server.

Die Index-Claim-Prüfung verwendet
`python3 scripts/check-forbidden-claims.py --self-test --check-index`.
Bestanden: 94 Selbsttest-Fälle und 115 geprüfte öffentliche Dateien im Index.

## Java-JAR und unabhängiger Client

Die Fixture wird separat ausschließlich gegen Paper-API und Java-Standard-API
kompiliert. Ihr JAR enthält keine FerrumC-Aufrufe und wird ohne Bytecode-Änderung
geladen. Die Socket-Abnahme prüft den tatsächlich versendeten Commandbaum,
Login-UUID, Uppercase-/Alias-/Namespace-Routen, Rechte und Autocomplete,
Listener-Reihenfolge, TabCompleter-/Executor-Fallback und Neustart. Completion
prüft Alias-Schreibweise, leere Argumente, Rechte, Fehler und anschließende
Recovery. Welt-/Scheduler-Aufrufe melden konkrete
Fehler; danach funktionieren Commands weiter. Unicode-Exceptions bleiben
begrenzte Remote-Fehler. Nachrichten an andere Spieler werden abgewiesen.

Zwei echte Startup-Fehlerpfade wurden geprüft: ein unverändertes Fixture-JAR
mit kollidierendem `spawn`-Alias und ein Fehler während `onEnable`. Beide
deaktivieren die reale Plugin-Instanz einmal und geben den Listener-Port frei.

`tests/blackbox/java-plugin.mjs` besteht zusätzlich mit unabhängigen
PrismarineJS-Decodern und zwei Clients. Geprüft werden Join-Prioritäten,
Command-Rechte, Alias, kanonische UUID, nicht-ASCII-Text, Literal- und echte
Plugin-Completion, `minecraft:ask_server`, leere Argumente, originale
Alias-Schreibweise, Emoji und UTF-16-Bereiche, Fehlerfeedback, Recovery und
Nachrichtenrouting. Eine Antwort auf dem
Gast-Stream bildet eine Barriere vor der Prüfung auf fremde Nachrichten.
Nach SIGTERM sind `enabled=1`, `disabled=1`, `join-count=2`, `quit-count=2`
und genau ein Disable-Log belegt.

Minecrafts NBT verwendet Modified UTF-8: der offizielle 1.21.8-Client ruft
bei `StringTag` `DataOutput.writeUTF` / `DataInput.readUTF` auf; auch SystemChat
verwendet diese NBT-Kette. PrismarineJS decodiert Emoji-Surrogatpaare dort als
sechs Ersatzzeichen. Der unabhängige Texttest prüft deshalb `Grüße Ω`.
Emoji-JSON-Transport und Rust-NBT-Roundtrip bestehen separat. Der korrekte
NBT-Encoder wurde wegen dieser Decodergrenze nicht verändert.

## Gameplay und Persistenz

Der unabhängige `smoke.mjs` bestand erneut mit den N02/I03/S02-Erweiterungen, einem
frischen Datenordner und vollständigem Prozessneustart. Status meldet
1.21.8/772; Login, Chunk-Streaming, Blockstates, Platzieren/Entfernen mit Acks,
Cross-Chunk-Edit und Reconnect funktionieren. Alle fünf erwarteten Positionen
stimmen nach Reconnect und Neustart: Stein, ausgerichteter Log, ausgerichtete
Treppe, ein entfernter Block und ein zweiter Stein über der Chunk-Grenze.
Beide Serverprozesse endeten sauber mit Code 0.

Der zusätzliche unabhängige `inventory-cursor.mjs` prüft eine volle Tasche
mit 2.304 Stein-Items und 13 Diamant-Markern. Er bestätigt Cursor-Erhalt bei
Close/Reopen, Truhenwechsel, veralteten IDs, parallelem Login derselben UUID,
Rejoin sowie zwei Prozessneustarts. Die Marker werden genau einmal eingezahlt.
Ein benannter Cursor wurde während der Verbindung geprüft; Neustartprüfungen
verwendeten am Basisstand Items ohne Komponenten. Alle drei Prozesse endeten
sauber mit Code 0, der Listener-Port war geschlossen und die frühere
Leave-Quarantäne-Warnung trat nicht mehr auf. Der damalige Vergleich mit dem
alten Binary zeigte erwartungsgemäß den Verlust von sieben Cursor-Items;
die unveränderte Abnahme bestand mit dem korrigierten Basisstand.

### S02 — produktive Komponenten-Persistenz

Spieler-Schema 3 verwendet `FCPS` mit genau 47 längengerahmten `FCIS`-Records:
46 Slots einschließlich Crafting-/Armor-/Offhand-Bereich plus Cursor. Alle
derzeit unterstützten Komponentenvarianten, opaque Bytes, Removals,
NBT-Reihenfolge, doppelte NBT-Namen und Floatbits bleiben erhalten. Das ganze
Player-Payload ist höchstens 1 MiB. JSON-Schemas 1/2 behalten bei der expliziten
Migration ihre historischen Normalisierungsregeln; Laden schreibt keine Daten
zurück. Komponenten, die ältere Writer ausgelassen haben, bleiben verloren.

World verwendet den versionierten Checked-Writer für Schilder und Truhen;
Storage/Sim/App verwenden jetzt Checked-Capture und atomar gestagtes Apply.
Ein fehlerhaftes Item verwirft die ganze Entity beziehungsweise das ganze
Capture/Load. Es wird kein leerer Ersatzcontainer zugelassen. Ein später
Capturefehler lässt Dirty-Masks, Scan-Fortsetzung, Journal und ID-Zähler
unverändert; eine volle Queue überträgt keine Verantwortung. Persistierte
Chunk-Koordinaten müssen zum Schlüssel passen. Die Prüfung erfolgt beim
Redb-Laden und zusätzlich bei Sim-Zulassung generischer Store-Ergebnisse.

Die bestandenen Abnahme-Fixtures umfassen alle Slot-/Cursor-Komponenten über
zwei echte Serverneustarts (`app/tests/player_component_persistence.rs`), Redb-Reopen mit
NaN-/Floatbit-Erhalt, explizite Legacy-Migration, fehlerhafte/trunkierte
Records, exakt passende und überschrittene Grenzen, falsche Schlüsselspalten
sowie frühe/späte Capture-/Commit-Fehler. Der erweiterte unabhängige
`inventory-cursor.mjs` bestand mit sechs verwalteten Prozessen; Name, Damage,
Removals, Armor, Offhand und Full-Bag-Cursor blieben über Rejoin/Neustarts erhalten.

Der neue `chest-component-persistence.mjs` bestand mit drei Prozessen und zwei
Neustarts: alle Truhentransfers erfolgen im Survival-Modus. Ein Marker mit Name,
Damage, Removals, doppelten NBT-Schlüsseln, NaN-Bitmustern und Signed Zero behält
exakt dieselben Slot-Bytes. Beide Truhen, alle Spielerslots und Cursor enthalten
insgesamt genau sieben Marker; ein veralteter Replay verändert nichts.
`player-inventory.mjs` bestand ebenfalls mit Links-/Rechtsklicks, Komponenten,
Equipment, Spectator-Regeln und voller Tasche. Alle fünf unabhängigen Scripts
sind verpflichtende CI-Gates, einschließlich `smoke.mjs` und `java-plugin.mjs`.
Die Chest-Snapshot-Prüfung vergleicht FCIS-Bytes, da Float-PartialEq identische
NaNs ablehnt und Signed-Zero-Änderungen übersieht.

Die kodierten Grenzen sind 16 KiB je Block-Entity, 2 MiB für alle Entity-
Positionen/Längen/Payloads eines Chunks und 3 MiB je vollständigem Chunk- oder
Overlayrecord. Entity-/Player-Decoder prüfen ihre geliehenen Payloadlängen
vor dem Kopieren. Der produktive Driver erfasst vier Overlays je Anfrage,
die Queue hat acht Slots, und der Worker schreibt ab 8 MiB wartenden
kodierten Overlaybytes beziehungsweise seinen Count-/Zeitgrenzen. Ein
fehlgeschlagener Save behält die zugehörige Bytebilanz bis zum erfolgreichen
Retry. Diese Caps begrenzen kodierte Bytes, nicht den Heap oder RSS.

Legacy-Infallible-Capture bleibt im experimentellen Scheduler. Crash-Atomizität
zwischen Player-, Overlay- und Journalrecords sowie Journal-Recovery bleiben
offen; normale erfolgreiche Saves/Barrieren ersetzen diesen Vertrag nicht.

## Release-Ressourcenbasis

Die [eingefrorene Release-Messung vom 10.10.2026](../tests/performance/results/2026-10-10/README.md)
verwendet `7333f404`, dieselbe flache Weltkonfiguration und 0/4/8 stehende
Clients in zwei Runden mit umgekehrter Modusreihenfolge. Bei acht Clients:
Rust-RSS nativ 21,48–21,70 MiB, Rust plus JVM mit der Bukkit-Fixture
108,41–108,72 MiB, Serverprozess-CPU 1,90–1,93 % eines logischen Kerns.
Die effektiven TPS aller Messfenster lagen bei 19,964–20,032.

Die verlinkten Rohdaten nennen Hardware, Binary-/JAR-/Image-Hashes, Configs
und Messdauer. Summiertes RSS kann gemeinsam benutzte Seiten mehrfach zählen;
Clients, Dateisystemcache und Docker-Overhead sind ausgeschlossen. Der vorhandene
Driver-Timer misst einen Abschnitt und keine vollständige Server-MSPT.
Es gab keine aktive Bewegung, Weltänderungen, Survival-Mechaniken oder Plugin-
Arbeitslast. Rust kann Ressourcen sparen; ein Vorteil gegenüber einem
gleichwertigen Paper-/Survival-Server ist durch diesen Test nicht belegt.
Die spätere N02/I03/S02-Implementierung wurde in dieser Messung nicht ausgeführt.

## Container und Pterodactyl

Das Runtime-Image `mchosting/ferrumc:v2-local` und das separate
`mchosting/ferrumc-installer:v2-local` wurden lokal gebaut. Bookworm erhält
die Rust-Build-glibc; die kopierte Java-21-JRE stammt aus dem kompatiblen
Jammy-Image. UID/GID 1000, FerrumC als PID 1, ein Allocation-Port abweichend
von 25565, Status/Client und SIGINT/SIGTERM wurden geprüft.

Das echte Egg-Installationsskript läuft als Root ohne Image-ENTRYPOINT mit
dem Wings-Aufruf `/bin/sh /mnt/install/install.sh`. Erstinstallation erstellt
Template und Datenordner. Reinstall erhält bestehende Konfiguration, Welt-
und Plugin-Dateien. Portfehler enden mit Code 64; `--help` funktioniert.

Die Java-Container-Abnahme ist reproduzierbar über
`python3 deploy/tests/container-java.py`. Sie verwendet den echten Installer,
Java21 und den unabhängigen Zwei-Client-Test im Image. Zusätzlich prüft sie
eingebettete Lizenzhinweise, API-/Adapterquellen und Plugin-Lifecycle/Persistenz
bei graceful Stop und Reinstall. Beide Images wurden nach N02/I03/S02 neu
gebaut; Java- und native Container-Abnahme bestanden erneut mit Exit 0.

Aktuelle lokale Image-IDs:

- Runtime: `sha256:863f35c87378e7aeb62a411066a1085810e589f75e168cdd42d00b8776f2e9c3`.
- Installer: `sha256:225a553c249631f418fd78f430516e1900d237617e2b780c98d04df5f8e70e97`.

Der alte Release-Runtime ist separat unter `mchosting/ferrumc:baseline-7333`
gesichert, mit denselben Binary-/Bridge-Hashes wie im Messlauf. Sein
wiederhergestellter amd64 Runtime-Manifest hat die ID `sha256:e0cdc5f1e24240dec778a81903ff25fcbacb6adce95eadeb5ed7db24000a5b78`;
der historische Messindex `b11b4e3a` enthält zusätzlich eine lokal nicht mehr
vorhandene Attestation. Die unveränderten Rohdaten behalten den historischen
Index, und die [Harness-Anleitung](../tests/performance/README.md) nennt die
verfügbare Runtime für neue Reproduktionsläufe. Kein neuer Performance-Lauf
wurde auf den aktuellen S02-Images durchgeführt.

## Verbleibende Grenzen

Die Fixture belegt die genannte Teilmenge. Scheduler, World/Inventory/Entity-
APIs, Bukkit-Rechtebaum, Broadcasts, Paper-Bootstrap, NMS/CraftBukkit und viele
Spielmechaniken fehlen. FerrumC läuft weiter im Offline-Modus; Spielernamen
sind keine authentifizierten Identitäten. Zusätzliche Inventar-Klickarten,
proaktive Viewer-Updates, periodische Player-Checkpoints, Journal-Recovery und
Survival bleiben im [Umsetzungsplan](V2_IMPLEMENTATION_PLAN.md) offen.

Zustandsändernde Handler-/Containertransaktionen werden nicht mitten in einer
Änderung abgebrochen. Ihr begrenzter Abschlussvertrag bleibt N06. Ein hart
beendeter Java-Prozess garantiert keine Disable-Hooks.

Die Client-Abnahme ist keine interaktive Vanilla-Client- oder Lastabnahme.
Panel/Wings wurden nicht tatsächlich installiert; Images wurden nicht in
eine Registry veröffentlicht. GitHub-CI-Ausführung ist von lokalen Prüfungen
zu unterscheiden. Der Basisstand `7333f404` bestand den vollständigen
[GitHub-Lauf 38046184757](https://github.com/jstrausd/ferrumc/actions/runs/38046184757);
dieser Lauf belegt die damaligen Cursor-/Completion-/Shutdown-Änderungen,
aber nicht die späteren N02/I03/S02-Erweiterungen. Deren CI-Ergebnis ist beim
jeweiligen Commit in
[GitHub Actions](https://github.com/jstrausd/ferrumc/actions/workflows/ci.yml)
nachzusehen. Kein bestehender Hosting-Server wurde geändert oder migriert.
