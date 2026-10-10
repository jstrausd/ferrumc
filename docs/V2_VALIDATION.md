# FerrumC v2 — Prüfbericht des Hosting-Forks

Stand: 10.10.2026. Fork: [jstrausd/ferrumc](https://github.com/jstrausd/ferrumc).
Ausgangspunkt: upstream `rework/v2-skeleton`, Commit
`8315da2c407a66ef591d2ef9de158aa74e7f6bc4`.
Prüfumgebung: Linux x86_64, Rust/Cargo 1.99.0, Java 21, Maven 3.9.11.

`main` enthält v2 und ist der einzige Branch im Fork. Der erste Hosting-PR
wurde integriert; vor dem Entfernen der übrigen 19 Fork-Branches wurde ein
vollständiges Git-Bundle samt Branchliste außerhalb des Checkouts gesichert.
Die Upstream-Branches wurden nicht verändert.

## Geprüfte Änderungen

- Play-Keep-alive mit exakten IDs, absoluten Fristen, fairer Bearbeitung und
  cancellation-sicherer Socket-/Chunk-Übergabe.
- Truhen-Klicks prüfen Fenster-/State-ID; der Driver vergleicht die letzte
  Chest-Sicht atomar. Veraltete Klicks verändern keine Items. Close-Pakete für
  andere Fenster schließen das aktuelle Fenster nicht.
- Rust 1.99.0 numerisch gepinnt. Bestehende Compiler-/Clippy-Diagnosen wurden
  crateweise korrigiert und einzeln committed. Die strenge Workspace-Prüfung
  ist lokal grün.
- Optionale Java-21-Brücke gegen die echte, timestamp-gepinnte Paper-1.21.8-API:
  gewöhnlicher `plugin.yml`-Loader, Lifecycle/Konfiguration, deklarierte Commands,
  Aliases/Namespaces, aktuelle Spieleridentität/-nachrichten und Join-/Quit-Events.
- Docker/Pterodactyl enthalten Java-Laufzeit, Adapter, Lizenzhinweise und
  entsprechende Quellen. CI verwendet `main` und verpflichtende Java- und
  unabhängige Client-Gates.

FerrumC bleibt der Rust-Spielserver; seine Welt und Netzwerkverbindungen
gehören der Rust-Laufzeit. Die Java-Brücke implementiert eine getestete
API-Teilmenge. Die Grenzen stehen im
[Adaptervertrag](../server/java-bridge/README.md) und
[ADR-0009](adr/0009-java-plugin-compatibility.md).
Es wurden keine generierten Protokolldateien geändert.

## Code-Prüfung

| Prüfung | Ergebnis |
| --- | --- |
| `cargo fmt --all --check`, `git diff --check` | Bestanden. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Bestanden, ohne CLI-Lint-Ausschlüsse. Drei dokumentierte trait-lokale Ausnahmen betreffen den von `async_trait` erzeugten `double_must_use`-Lint. |
| `cargo test --workspace --locked --no-fail-fast` | 1.975 Tests in 146 Gruppen bestanden, einschließlich Doc-Tests und nativer Plugin-Fixtures. Ein expliziter Java-Artefakt-Test ist standardmäßig ignoriert und wurde separat erfolgreich aufgerufen. |
| App-Unit-Abnahme | 208 Tests bestanden; Keep-alive, Truhen-State und Bridge-Parser/Aktor-Fehlerpfade enthalten. |
| Rust/JAR-Socket-Abnahme | `cargo test -p ferrumc-app --locked --test java_compat -- --ignored` bestanden; fehlende Java-/JAR-Artefakte führen bei Aufruf zum Fehler. Dieser Aufruf ist obligatorisch in CI. |
| `cargo xtask generate --check` | Bestanden; eingecheckter Protokollcode aktuell. |
| `cargo metadata --no-deps --locked --format-version 1` | Bestanden. |
| Java `mvn -B clean verify` | 11 Parser-/Archiv-Unit-Tests und sechs echte Prozessintegrationstests bestanden; keine übersprungenen Tests. |

Die Java-/Rust-Brücke wurde unabhängig auf Prozessbesitz, Reaping, begrenzte
Frames/Queues/Metadaten, IDs, absolute Fristen inklusive Warteschlange,
Cancellation, Startup-Rollback und Empfängergrenzen geprüft. Keine offenen
Blocking-Findings im implementierten Subset. Commandkonflikte werden auch
bei unterschiedlicher Groß-/Kleinschreibung vor Client-Annahme abgewiesen.

Die Index-Claim-Prüfung verwendet
`python3 scripts/check-forbidden-claims.py --self-test --check-index`.
Bestanden: 94 Selbsttest-Fälle und 115 geprüfte öffentliche Dateien im Index.

## Java-JAR und unabhängiger Client

Die Fixture wird separat ausschließlich gegen Paper-API und Java-Standard-API
kompiliert. Ihr JAR enthält keine FerrumC-Aufrufe und wird ohne Bytecode-Änderung
geladen. Die Socket-Abnahme prüft den tatsächlich versendeten Commandbaum,
Login-UUID, Uppercase-/Alias-/Namespace-Routen, Rechte und Autocomplete,
Listener-Reihenfolge und Neustart. Welt-/Scheduler-Aufrufe melden konkrete
Fehler; danach funktionieren Commands weiter. Unicode-Exceptions bleiben
begrenzte Remote-Fehler. Nachrichten an andere Spieler werden abgewiesen.

Zwei echte Startup-Fehlerpfade wurden geprüft: ein unverändertes Fixture-JAR
mit kollidierendem `spawn`-Alias und ein Fehler während `onEnable`. Beide
deaktivieren die reale Plugin-Instanz einmal und geben den Listener-Port frei.

`tests/blackbox/java-plugin.mjs` besteht zusätzlich mit unabhängigen
PrismarineJS-Decodern und zwei Clients. Geprüft werden Join-Prioritäten,
Command-Rechte, Alias, kanonische UUID, nicht-ASCII-Text, Literal-Completion,
Fehlerfeedback, Recovery und Nachrichtenrouting. Eine Antwort auf dem
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

Der unabhängige `smoke.mjs` bestand gegen den finalen Debug-Server mit einem
frischen Datenordner und vollständigem Prozessneustart. Status meldet
1.21.8/772; Login, Chunk-Streaming, Blockstates, Platzieren/Entfernen mit Acks,
Cross-Chunk-Edit und Reconnect funktionieren. Alle fünf erwarteten Positionen
stimmen nach Reconnect und Neustart: Stein, ausgerichteter Log, ausgerichtete
Treppe, ein entfernter Block und ein zweiter Stein über der Chunk-Grenze.
Beide Serverprozesse endeten sauber mit Code 0.

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
bei graceful Stop und Reinstall.

## Verbleibende Grenzen

Die Fixture belegt die genannte Teilmenge. Scheduler, World/Inventory/Entity-
APIs, Bukkit-Rechtebaum, Broadcasts, Paper-Bootstrap, NMS/CraftBukkit und viele
Spielmechaniken fehlen. FerrumC läuft weiter im Offline-Modus; Spielernamen
sind keine authentifizierten Identitäten. Cursor-Overflow, normale Inventar-
Klicks, Item-Komponenten, proaktive Viewer-Updates und Survival bleiben im
[Umsetzungsplan](V2_IMPLEMENTATION_PLAN.md) offen.

Zustandsändernde Handler-/Containertransaktionen werden nicht mitten in einer
Änderung abgebrochen. Ihr begrenzter Abschlussvertrag bleibt N06. Ein hart
beendeter Java-Prozess garantiert keine Disable-Hooks.

Die Client-Abnahme ist keine interaktive Vanilla-Client- oder Lastabnahme.
Panel/Wings wurden nicht tatsächlich installiert; Images wurden nicht in
eine Registry veröffentlicht. GitHub-CI-Ausführung ist von lokalen Prüfungen
zu unterscheiden. Kein bestehender Hosting-Server wurde geändert oder migriert.
