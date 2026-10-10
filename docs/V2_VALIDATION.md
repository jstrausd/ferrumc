# FerrumC v2 — Prüfbericht des Hosting-Forks

Ausgangscode: `8315da2c407a66ef591d2ef9de158aa74e7f6bc4`,
upstream `rework/v2-skeleton`. Fork: [jstrausd/ferrumc](https://github.com/jstrausd/ferrumc).
Prüfumgebung: Linux x86_64, Rust/Cargo 1.99.0. Stand: 10.10.2026.

Die Änderungen betreffen die App-Crate sowie Dokumentation und Deployment.
Es wurden keine generierten Protokolldateien geändert.

## Ausgangsprüfung

`cargo test --workspace --locked --no-fail-fast` lief vor den Änderungen
erfolgreich durch, einschließlich Doc-Tests und dynamischer Plugin-Fixtures.
`cargo xtask generate --check` bestätigte den eingecheckten Protokollcode.
Die Claim-Prüfung bestand mit 94 Selbsttest-Fällen.

## Bekannter Upstream-Blocker

Die vorgeschriebene strenge Workspace-Prüfung

```sh
cd server
cargo clippy --workspace --all-targets --locked -- -D warnings
```

scheitert mit Rust 1.99.0 bereits in unverändertem Code:
`ferrumc-world/src/paletted_container.rs:91`, `clippy::manual_bit_width`.
Eine fokussierte App-Prüfung findet außerdem den neuen Lint
`clippy::assert_is_empty` in bestehenden Tests. Weitere Compilerwarnungen
betreffen `AtomicU64::fetch_update` in `ferrumc-plugin-abi-sys` und einen
unbenutzten Test-Import in `ferrumc-testkit`.

Die breite CI ist damit noch nicht grün. Die erste Feature-Änderung ersetzt
nicht nebenbei APIs und Tests in weiteren Crates. Der Toolchain-/Lint-Abgleich
ist als H03 im [Umsetzungsplan](V2_IMPLEMENTATION_PLAN.md) erfasst.

## Finale Code-Prüfung

| Prüfung | Ergebnis |
| --- | --- |
| `cargo test --workspace --locked --no-fail-fast` | Bestanden: 1.958 Tests in 144 Testgruppen, keine Fehler oder ignorierten Tests; einschließlich Doc-Tests und nativer Plugin-Fixtures. |
| Play-Liveness-Regressionsabdeckung | 192 App-Unit-Tests und drei Socket-Regressionsfälle bestanden; diese sind in der Workspace-Summe enthalten. |
| `cargo fmt --all --check`, `git diff --check` | Bestanden. |
| `cargo xtask generate --check` | Bestanden; eingecheckter Protokollcode aktuell. |
| `python3 scripts/check-forbidden-claims.py --self-test --check-index` | Bestanden: 94 Selbsttest-Fälle und 114 geprüfte Dateien im Index. |
| Fokussiertes Clippy | Bestanden mit dem unten genannten Ausschluss für bestehende Test-Lints. |

Die fokussierte App-Prüfung lautet:

```sh
cd server
cargo clippy -p ferrumc-app --all-targets --no-deps --locked -- \
  -D warnings -A clippy::assert_is_empty
```

Sie ersetzt die noch fehlschlagende strenge Workspace-Prüfung nicht.
Die Änderung wurde außerdem unabhängig auf faire Bearbeitung von Eingaben,
absolute Fristen und die Übergabe der Chunk-Tickets bei Timeout geprüft.
Deterministische Tests decken bereits zugestellte und noch nicht zugestellte
Chunk-Antworten sowie erfolgreiche und fehlgeschlagene durable Freigabe ab.

## Unabhängiger Client und Laufzeit

Das Release-Image `mchosting/ferrumc:v2-local` wurde lokal gebaut. Der Container
lief als UID 1000 mit FerrumC als PID 1. Ein Allocation-Port abweichend von
25565 war erreichbar; Status-Ping meldete Protokoll 772. Der vorhandene
`tests/blackbox/smoke.mjs` bestand gegen den Container: Offline-Login,
Chunk-Streaming, Platzieren/Entfernen mit Blockstates und Rejoin.

Ein zweiter, vom Smoke-Skript verwalteter Lauf verwendete das Release-Binary
aus dem Image und einen frischen Datenordner:

```sh
cd tests/blackbox
MC_MANAGE_SERVER=1 MC_SERVER_BIN=/path/to/ferrumc \
  MC_RUN_DIR=/path/to/empty-run-dir node smoke.mjs
```

Auch dieser Lauf bestand. Nach einem vollständigen Prozessneustart wurden
alle fünf erwarteten Blöcke einschließlich ausgerichteter Logs/Treppen und
eines entfernten Blocks durch den unabhängigen Decoder bestätigt.

## Pterodactyl-Installer

Das separate Image `mchosting/ferrumc-installer:v2-local` wurde mit dem
Dockerfile-Target `installer` gebaut. Seine Metadaten bestätigen `User=0:0`
und keinen `ENTRYPOINT`. Das echte Egg-Skript lief mit dem Wings-Aufruf
`/bin/sh /mnt/install/install.sh`, ohne User- oder Entrypoint-Override:
Erstkopie des Templates, Datenordner und UID/GID 1000 wurden geprüft.
Erneute Installation erhielt bestehende Konfiguration, Welt- und Plugin-Dateien.

Der Laufzeit-Container erhielt SIGTERM und SIGINT direkt und meldete jeweils
`shutdown complete`. Der Datenordner ließ sich erneut öffnen. Ein ungültiger
Port wurde mit Exit-Code 64 abgewiesen; `--help` funktionierte.
Die Shell-Prüfung umfasste außerdem Portgrenzen, führende Nullen, lange und
nichtnumerische Werte sowie die unveränderte Weitergabe separater CLI-Argumente.

## Grenzen der Abnahme

Der Client-Smoke verwendet die unabhängigen PrismarineJS-Decoder für
Minecraft Java 1.21.8 / Protokoll 772. Er ist keine interaktive Abnahme mit
dem offiziellen Vanilla-Client und kein Last-/Kapazitätsnachweis.

Das Egg wird als JSON und mit seinem echten Installationsskript geprüft;
der Container wird lokal betrieben. Eine Installation in Panel/Wings und
ein veröffentlichtes Registry-Image sind noch nicht durchgeführt.
Es wurde kein bestehender Hosting-Server geändert oder migriert.

Die Play-Deadline unterbricht nur abbrechbare Socket-/Chunk-Waits.
Zustandsändernde Handler und Containertransaktionen werden nicht mitten in
einer Inventaränderung abgebrochen; ihre Wartezeiten bleiben Folgearbeit N06.
