# D0: Operator-Rechte und administrativer Zugang

Stand: 10.10.2026. Zielversion: Minecraft Java 1.21.8, Protokoll 772.
Ergänzung zum [Survival-Lieferplan](SURVIVAL_DELIVERY_PLAN.md).
Dies ist eine native-only Offline-Teilmenge; D0 und Paper-Kompatibilität bleiben
offen. Eine Umschaltung des GameMode belegt keinen spielbaren Survival.

## Implementierter Vertrag

Bei konfiguriertem `world_dir` und ohne Java-Brücke stehen `/op <name>` und
`/deop <name>` zur Verfügung. Ein Ziel ist genau ein case-sensitiver Offline-Name
mit 1–16 ASCII-Buchstaben, Ziffern oder Unterstrichen. Selektoren,
Mehrfachziele und Online-Profilauflösung fehlen. Die Offline-UUID wird aus
dem Namen nach dem bestehenden Serververtrag bestimmt.

Die konfigurierte `ops`-Liste liefert Seeds. Dauerhafte UUID-bezogene Overrides
in `world_dir/operators.json` können sie gewähren oder widerrufen. Ein
gespeicherter Widerruf überlebt auch einen Neustart mit demselben Seed in der
Konfiguration. Dies ist FerrumC-Schema 1, kein Vanilla-`ops.json`; maximal
1.024 Config-Seeds und zusätzlich 1.024 Overrides sind zulässig, Records haben
höchstens 128 KiB. Unbekannte Felder, Schemafehler,
ungültige Namen, doppelte Identitäten und übergroße Daten verhindern den Start
ohne Umschreiben der Datei.

Spieler brauchen Level 3 und tatsächliche aktuelle Operator-Mitgliedschaft.
Ein hoher `default_permission_level` allein gewährt keine Rechte zur
Operator-Mutation. Nach Erwerb des serialisierten Mutationsrechts wird die
Identität erneut geprüft; ein inzwischen widerrufener Issuer darf keine später
wartende Mutation ausführen. Rust-Rechte und Commandgraph lesen die gemeinsame
Live-Quelle. Coalesced Revisionen aktualisieren die Commandgraphs verbundener
Spieler über eine eigene Quelle in der fairen Play-Rotation.
Explizite Defaultlevel und gesondert konfigurierte Bypass-Grants bleiben bei
Deop erhalten; widerrufen wird die Operator-Mitgliedschaft.

Die echte CLI-Konsole akzeptiert dieselben zwei Befehle ohne Spieleridentität.
Ein dedizierter Lesethread liefert höchstens 16 wartende Zeilen mit jeweils
256 Bytes; Übergröße/ungültiges UTF-8 werden abgewiesen. EOF deaktiviert die
Inputquelle. `RunningServer.execute_admin_command` bietet den gleichen lokalen
administrativen Vertrag für Einbettung und Tests. Dieser Zugang ermöglicht
Wiederherstellung, wenn alle Spieler ihre Operator-Rechte verloren haben.

## Speicherung, Cancellation und Stop

Eine akzeptierte Änderung gehört einem Server-Task und hält die Serialisierung
bis nach Speicherung und Veröffentlichung. Abbruch des wartenden Callers
verwirft sie nicht. Noch nicht akzeptierte Spieleränderungen beobachten die
absolute Play-/Keepalive-Frist und Stop; die CLI beobachtet OS-Signale und
internen Stop auch während einer Änderung.

Die Speicherung läuft über `spawn_blocking`: eindeutige exklusiv erzeugte
Staging-Datei, File-fsync, Rename und Directory-fsync. Erst danach erscheint
der neue Live-Zustand. Ein Fehler vor Rename erhält die vorherigen Rechte.
Bei Fehler nach erfolgreichem Rename ist die Haltbarkeit ungewiss: Operator-
Identitäten werden gesperrt, Mutationsadmission geschlossen, koordinierter
Stop ausgelöst und dessen Ergebnis als Fehler gemeldet. Dies ist kein
Crash-atomarer Vertrag zwischen Operator-, Spieler- und Weltdaten.

Stop schließt Mutationsadmission synchron vor jedem Await und signalisiert
die Connections vor dem Drain. Die bereits akzeptierte Änderung darf fertig
werden; nur wartende Änderungen werden abgewiesen. Danach läuft der bestehende
Connection-/Simulation-/Storage-Shutdown. Ein Test-Fault nach echtem Rename
prüft die fail-closed Behandlung eines Directory-fsync-Fehlers; er ist kein
realer Stromausfalltest.

## Referenz und Abweichungen

Gelesen: Paper 1.21.8 Build 60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`, vollständige angewandte Basis und
Source-/Feature-Patches: `OpCommand`, `DeOpCommands`, `Commands.hasPermission`,
`CommandSourceStack.hasPermission`, `PlayerList.op/deop/sendPlayerPermissionLevel`
und `CraftPlayer.isOp/setOp`.

Paper nutzt nominal Level 3, aber die effektive Commandprüfung bezieht Bukkit-
Permissions und Konfiguration ein. FerrumCs zusätzliches Identitäts-Gate ist
bewusst strenger. Paper verarbeitet GameProfiles, liefert zielbezogene
Suggestions und sendet bei Änderungen Client-Permission-EntityEvents 24–28,
recalculiert Bukkit-Permissions und sendet Commands. Deop prüft außerdem
Whitelist-Zugang erneut. Dieser Schritt implementiert nur den beschriebenen
Offline-Einzelziel-, Rust-Rechte- und Commands-Vertrag; EntityEvents,
Bukkit-Recalc, Whitelist, vollständige Suggestions und signed commands fehlen.
Es gab keinen Vergleichslauf mit einem gestarteten Paper-Server.

Die Java-Brücke hat separat einen getesteten `player_state`-RPC erhalten,
der den Rechte-Snapshot bereits gespeicherter Player-Proxies ersetzt, ohne
Join-/Quit-Events zu erzeugen. Die Rust-Mutation aktiviert ihn noch nicht.
Bei Java-Konfiguration bleiben Operator-Mutationen deshalb ausdrücklich
gesperrt, bis Ziel-Refresh, Bestätigung und verlorene Antwort als gemeinsamer
Rust-/Java-Vertrag implementiert sind. Bukkit-Defaults, Attachments und der
allgemeine Permission-Baum bleiben zusätzlich offen.

## Abnahme

Die App-Tests prüfen autorisierte und abgewiesene Befehle, tatsächliche
GameMode-Pakete, Commandgraphs bei zwei Clients, Seed-Widerruf und Offline-Grant
nach Redb-Neustart sowie lokale Wiederherstellung. Unit-Regressionen prüfen
FIFO-Rechteentzug, Defaultlevel, Bounds/Parserfehler, fehlerhafte Ersetzung,
post-Rename-Fehler, accepted Cancellation, queued Stop, absolute Keepalive-Frist,
Java-Guard und faire Verarbeitung dauernder Revisionen.

Der unabhängige [Node-Client](../tests/blackbox/operator-commands.mjs) verwendet
Prismarine für Wire-/Commandgraph-/NBT-Dekodierung. Drei echte Serverprozesse
auf derselben Welt prüfen Live-Rechte, abgewiesene Commands, tatsächliche
GameMode-Events, zwei Neustarts, die echte stdin-Konsole, EOF und dauerhaft
gespeicherte CLI-Grants/-Widerrufe. Jeder Prozess muss nach SIGTERM mit Code 0
ohne Signal enden. Die verbindlichen ausgeführten Ergebnisse werden im
[v2-Prüfbericht](V2_VALIDATION.md) festgehalten; eine Client-Fixture erweitert
keine Survival- oder allgemeine Plugin-Abnahme.
