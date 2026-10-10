# N03 — serverautoritatives Teleport-ACK

Zielversion: Minecraft Java 1.21.8 / Protokoll 772. Produktionsänderungen
beschränken sich auf `ferrumc-app`; keine generierten Protokolldaten oder
Session-/Sim-Innereien werden geändert. Dieses Ticket aktiviert den vorhandenen
Join-/Teleport-Pfad und schützt nachfolgende Clientpositionen.

## Referenz und Vertrag

Paper 1.21.8 Build 60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`, vollständige lokal erzeugte
`paper-server/src/minecraft/java/net/minecraft/server/network/ServerGamePacketListenerImpl.java`:

| Symbol | Vollständig gelesener Vertrag |
| --- | --- |
| `handleAcceptTeleportPacket` (743–770) | Threadgebunden. Nur die aktuelle ID wird betrachtet. Aktuelle ID ohne ausstehende Position beendet die Verbindung; abweichende IDs werden ignoriert. Passende ACK setzt die bereits autoritative Position, aktualisiert Last-Good/Dimensions-/Chunkzustand und löscht die ausstehende Position. |
| `handleMovePlayer` (1398–1710) | Erst Wertevalidierung/Loaded-/Immobile-Prüfung. Bei ausstehendem Teleport wird nur Rotation angepasst; normale Positions-, Kollisions-, MoveEvent- und Bewegungseffekte bleiben aus. Die vollständige Methode einschließlich Reentrancy und Teleportkorrekturen wurde gelesen. |
| `updateAwaitingTeleport` (1731–1750) | Ausstehende Position bleibt wirksam. Der Vanilla-Resend nach mehr als 20 Ticks ist in Paper ausdrücklich deaktiviert. |
| `teleport` / `internalTeleport` (1793–1880) | Bukkit-Teleportevent vor Mutation, Cancellation/änderte Destination, relative Flags, removed-player/NaN-Schutz. Intern: ID erhöhen, beim Erreichen `Integer.MAX_VALUE` auf null setzen, Serverposition sofort setzen, erwartete Position und Bewegungsbaseline aktualisieren, Positionpacket senden. |

Zusätzlich gelesen: ursprüngliche Mache-Basis, der vollständige relevante
Sourcepatch `patches/sources/net/minecraft/server/network/ServerGamePacketListenerImpl.java.patch`
und Featurepatch
`patches/features/0030-Optimise-collision-checking-in-player-move-packet-ha.patch`.
Letzterer deaktiviert ausdrücklich den 20-Tick-Resend wegen Clients mit RTT
über 1000 ms. FerrumC erfindet weder diesen Resend noch eine neue Teleportfrist.

FerrumC weist IDs am einzigen Verbindungseigentümer zu: Join, inline `/spawn`
und über den Sessionkanal geroutete `/tp`, Pluginteleports und Korrekturen.
Ein neuer Teleport ersetzt den ausstehenden; alte IDs können ihn nicht
freigeben. Die einzige gültige aktuelle ACK löscht das Positiongate. Aktuelle
ID ohne Pending beendet die Verbindung über den normalen Save-/Leave-Pfad.
Look-only und die Lookkomponente gültiger kombinierter Bewegung werden bei
Pending weiter zugelassen, ohne Position/Move-Pluginbaseline/Streamziel zu ändern.

## Reihenfolge und begrenztes Ownership

Ein `/tp`-Driverreply beweist die bereits erfolgte begrenzte Shardadmission
und die zuvor eingereihte eigene Positionssynchronisation. Der alte komplette
Frame-Drain konnte nach diesem Reply eine im selben TCP-Write nachfolgende
Bewegung vor dem Outbound-Sync ausführen. N03 friert vor jeweils einer
nicht-Keep-Alive-Body einen endlichen FIFO-Präfix aus höchstens 256 bereits
bereiten Sessionausgaben ein. Jede Ausgabe behält ihre ursprüngliche
Criticality/Priorität und einen eigenen Flushturn. Danach läuft genau die
gehaltene Body; neue Produktion verlängert den Präfix niemals.

Eine unabhängige Reviewprüfung fand einen fehlenden Flush nach der Freigabe
dieser Body: Der einzelne FIFO-Output war geflusht, seine anschließend vom
Pump erzeugte Tabresponse oder inline `/spawn`-Sync blieb jedoch im Writer.
Der Branch flush't jetzt zusätzlich diese Antwort unter derselben absoluten
Playdeadline, bevor er endet. Der erste separate FIFO-Messageflush bleibt
erhalten; mehrere Sessionnachrichten werden nicht zusammengezogen.

Zusätzliche Retention: höchstens 256 `OutboundMessage`-Werte plus eine bereits
von den bestehenden Frame-/Dekompressionslimits begrenzte Body. Packetgrößen
variieren; dies ist keine RSS-Angabe. Es werden keine neuen Channels,
globalen Zustände, Weltmutationen im Netzwerk oder Storagehandles eingeführt.
Shutdown/absolute Playdeadlines behalten Select-Priorität. Eine exakte
Keep-Alive-Body am Eingangskopf darf das Positionfence umgehen; falsche,
unvollständige oder nachgestellte Daten laufen durch die strikte Verarbeitung.
Eine Teleportbestätigung darf das Fence nicht überholen: sonst könnte eine
alte Bestätigung vor der bereits akzeptierten neueren `/tp`-Position liegen.

Bei EOF, Stop oder Deadline spiegeln die gehaltenen FIFO-Ausgaben und höchstens
256 weitere Channelnachrichten bereits eingereihte autoritative Teleports und
Gamemodeänderungen vor dem Save. Kein Client-ACK ist für das Speichern der
Serverdestination nötig. Noch nicht eingereihte parallele Driverbefehle werden
damit nicht nachträglich als bereits gespeicherte Mutation zugesichert.

Das ACK-Gate schützt **bereits vom Verbindungseigentümer issued Teleports**.
Driver-Syncs, die vor dem einmaligen held-body-Fence bereits im Channel stehen,
werden vor dieser Body ausgegeben; dies schützt insbesondere pipelinierte
Bewegung nach dem eigenen Command-/Pluginreply. Ein externer Teleport kann
jedoch während der eingefrorenen Präfix-Flushturns später parallel eingereiht
werden. Fairnessbedingt läuft die bereits gehaltene Body vor diesem neuen
Channelabschnitt. Der App-Snapshot macht diese globale Welt-/Tick-Reihenfolge
nicht atomar und verspricht dafür keine allgemeine Bewegungsautorität.
**NET-06.2** muss diesen Fall mit einem gemeinsamen Epoch-/Move-Admission-Vertrag
über Producer und Consumer absichern; NET-06 bleibt deshalb insgesamt teilumgesetzt.

## Prüfungen

- **RED**: echter Socket vor Aktivierung, Join-Sync ohne ACK, Bewegung auf
  `(12,64,9)`, drivergebundenes Time-Reply und frischer Sim-Snapshot. Alter
  Stand übernimmt die Bewegung statt `(8,64,8)`; Test schlägt reproduzierbar
  fehl. Log `/tmp/ferrumc-n03-red.log`.
- **GREEN**: Rohclient mit absichtlich deaktivierter Auto-ACK: fehlende/falsche
  initiale ACK; passende ACK + Bewegung; `/spawn` ersetzt Join; `/tp` + Bewegung
  im selben TCP-Write; zweites `/tp` + alte ACK + Bewegung in einem Write;
  Replay der aktuellen ID stoppt vor pipeliniertem Sentinel; nicht bestätigte
  Teleportdestination und erlaubte Lookänderung über kompletten redb-Neustart.
- Reine ID-Tests: stale/negative/extreme IDs, aktueller Replay, genau ein
  Pending, neuer Teleport, Wrap auf null mit unveränderten übrigen Wirefeldern.
- Produktions-Pumpharness ohne Socket: exakter Keep-Alive-Head wird bei ständig
  bereitstehender Ausgabe mit/ohne Teleport sofort verarbeitet; ein vollständig
  gefüllter 256-Präfix wird pro Turn durch neue Produktion ersetzt und trotzdem
  nach endlich vielen Turns vor gehaltenem Input/anschließendem Echo beendet.
- Produktions-Handlerprobe: Pending position-only und ein unsicherer kombinierter
  Move routen nichts; gültiger Combinedmove routet ausschließlich Look. Position,
  Streamziel, Pluginbaseline und Callbackzahl bleiben unverändert. Separater
  Teardownhelper-Vertrag spiegelt einen bereits gehaltenen und anschließend im
  normalen Channel eingereihten Teleport in FIFO-Reihenfolge vor dem Save.
- **Review-RED/GREEN**: echte Socketfixture mit `ticks_per_second=1`,
  `io_timeout_secs=1`, `chunk_stream_interval_ms=60000` und
  `keep_alive_interval_ms=15000`. `/tp` und gehaltene Tabrequest werden in
  einem Write gesendet. Nach dem Empfang des `/tp`-Syncs sendet der Client
  nichts mehr; vorher endet die Verbindung nach 1,08 Sekunden ohne Tabresponse
  (`/tmp/ferrumc-n03-flush-red.log`, Exit 101). Nach dem Fix erreichen sowohl
  gehaltene Tabresponse als auch ein separat geprüftes gehaltenes `/spawn`
  unmittelbar den Socket. Ein TPS verhindert, dass der periodische
  20-Tick-UpdateTime-Flush den Defekt zufällig kaschiert.
- Bestehende normale Testclients bestätigen Join genau einmal; gelesene
  Joinframes bleiben für Assertions verfügbar. Spätere konsumierte Syncs werden
  wie bei einem normalen Client bestätigt. Securitytests benutzen ausdrücklich
  den unabhängigen unbestätigten Loginpfad.

Abschließende lokale Ergebnisse am 10.10.2026:

| Prüfung | Ergebnis / Log |
| --- | --- |
| `cargo test --locked -p ferrumc-app --all-targets` nach Reviewfix | **357 bestanden, 0 fehlgeschlagen, 1 ignoriert**, 38 Gruppen; `/tmp/ferrumc-n03-app-tests-reviewed.log`. Der ignorierte Java-Kompatibilitätstest wurde durch dieses Ticket nicht separat ausgeführt. |
| Exakte Bit-Assertions und Review-Flushfixture | **6 Sockettests bestanden**; `/tmp/ferrumc-n03-flush-green.log`. |
| `cargo clippy --locked -p ferrumc-app --all-targets -- -D warnings` nach Reviewfix | Bestanden; `/tmp/ferrumc-n03-clippy-reviewed.log`. |
| Gezielter `rustfmt --edition 2021 --check` und `git diff --check` | Bestanden für die eigenen App-Dateien. |

Diese lokalen Tests beziehen sich auf das vorbereitete N03-Diff. Root prüft
Integration, unabhängige Abnahme und veröffentlicht den Stand; hier wird kein
GitHub-CI-Ergebnis oder Deployment behauptet.

## Grenzen und verbleibende Tickets

Diese Quellenprüfung und FerrumC-Fixtures sind kein Laufzeit-Differenzialtest
gegen einen gestarteten Paper-Server. Die IDs und aktuellen/stalen ACK-Zweige
folgen der gelesenen Quelle. Bestehende FerrumC-Grenzen bleiben ausdrücklich:
Loaded/Immobile-/Dimension-/RelativeTeleport-Verhalten, vollständige Physik,
Rotationsnormalisierung, Bukkit `PlayerTeleportEvent`/`PlayerMoveEvent` und
Cancel/Reentrancy-Brücke sind kein neu erfüllter Vertrag dieses Tickets.
Native observe-only MoveEvents bleiben auf bestehender Blockgranularität.
Paper-spezifische Kickevents fehlen; FerrumC schließt den aktuellen-ID-Replay
mit klassifiziertem `ProtocolViolation` und normalem persistenten Teardown.
Native Pluginteleports verwenden den bereits vorhandenen gerouteten Syncpfad;
unveränderte Java-Plugins bekommen dadurch keine neue Teleport-API.
