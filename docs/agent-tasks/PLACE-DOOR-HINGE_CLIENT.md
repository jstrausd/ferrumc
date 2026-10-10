# Task: PLACE-DOOR-HINGE_CLIENT — BLK-01.1

## Auftrag und Eigentum

Unabhängige Blackbox-Abnahme des bereits implementierten Türscharnier-
Teilvertrags. Dieser Task besitzt ausschließlich
`tests/blackbox/door-hinge.mjs` und dieses Dokument. Keine Produktionsänderung,
kein Cargo-Build, keine Änderung bestehender Tests und keine Git-Mutation.
Root friert das getestete Serverbinary vor dem Lauf ein.

## Referenz und unabhängige Erwartungen

Minecraft Java 1.21.8, Protokoll 772. Quelle: Paper 1.21.8 Build 60,
Commit `29c8822d90899c89d2689338e81a98f690bcba12`, vollständig angewandte
`DoorBlock.getHinge`, `getStateForPlacement` und `setPlacedBy`,
`paper-server/src/minecraft/java/net/minecraft/world/level/block/DoorBlock.java`.
Der relevante Placementpfad und seine Abweichungen stehen in
[PLACE-DOOR-HINGE.md](PLACE-DOOR-HINGE.md).

Die zwölf isolierten Platzierungen verwenden explizite Property-Goldens:

| Spieler-Yaw / Facing | Cursor 0.25 | Cursor 0.5 | Cursor 0.75 |
| --- | --- | --- | --- |
| 0 / Süd | right | left | left |
| 90 / West | right | left | left |
| 180 / Nord | left | left | right |
| 270 / Ost | left | left | right |

`cursorX` und `cursorZ` besitzen jeweils den angegebenen Wert; angeklickt wird
die Oberseite des wirklichen Tragblocks, `cursorY=1`. Beide Hälften müssen
gleiches Facing/Hinge und `open=false`, `powered=false` besitzen. Ein weiterer
Fall platziert zuerst eine Birchentür und dann eine Eichentür daneben: die
untere Birchentürhälfte links von der südgerichteten Eichentür erzwingt
`hinge=right` trotz des Mittelpunkt-Cursors. Der ursprüngliche Zustand der
Birchentür bleibt im Restart-Golden enthalten.

Der Test importiert keinen FerrumC-Code. `minecraft-protocol` schreibt und
decodiert Pakete, `minecraft-data`/`prismarine-block` bestimmen die eindeutigen
State-IDs der Goldens, `prismarine-chunk` liest die Restart-Chunks. Die
Abhängigkeiten sind die bestehenden Pins unter `tests/blackbox/package.json`.

## Tatsächlicher Clientablauf

Ein frischer isolierter World-Ordner und ein eigener Loopback-Port (Standard
25717, `MC_PORT=0` für einen automatisch gewählten Port) verhindern die Nutzung
eines vorhandenen Servers. Die Demo-Plugins sind ausgeschaltet.

Zwei unabhängige Offline-Clients bleiben während der Platzierungen verbunden.
Der Actor erhält Türen durch echte `set_creative_slot`-Pakete samt überprüftem
Slot-Echo. Ein Operator-Teleport bringt ihn innerhalb der Reichweite;
zugehörige Teleport-ACKs gehen vor Look/Placement raus. Ein echtes Look-Paket
setzt den Yaw. Danach sendet der Client `block_place` (der Prismarine-Name
des `use_item_on`-Pakets), mit explizitem Cursor und Action-Sequence.
Actor und Observer lesen vor jedem Klick den tatsächlichen Tragblock aus dem
unabhängig decodierten Chunk und müssen dort `grass_block` beobachten.
Es gibt keinen `/setblock`-Ersatz für die Türplatzierung.

Jeder Fall wartet auf Action-ACK sowie echte Blockupdates für beide Hälften
auf Actor und Observer. Der erste Prozess wird mit verbundenen Clients per
SIGINT beendet; Exitcode 0 und `shutdown complete` sind Pflicht. Derselbe
Binarypfad und dieselbe World-DB werden erneut gestartet. Zwei frische Clients
lesen alle 28 erwarteten Half-States ausschließlich aus neu decodierten
Chunks. SHA-256 des Binaries wird vor beiden Starts und am Ende verglichen.
Ein zweiter SIGINT muss ebenfalls sauber abschließen.

Alle Wartebedingungen haben begrenzte Timers. Zeitablauf ersetzt keine
Paket- oder Prozessbeobachtung. Fehler führen zu Exitcode 1 und begrenzter
Prozessbereinigung mit SIGKILL als Fehler-Fallback; dieser Fallback zählt
niemals als bestandener Shutdown. Logs, DB und JSON-Ergebnis bleiben zur
Nachprüfung im ausgegebenen temporären Ordner.
Auch der kurze Portreservierungs-Listener wird bei Listenfehlern oder Timeouts
im `finally` abgebrochen und geschlossen; fremde Probeverbindungen werden
sofort zerstört, damit sie dessen Cleanup nicht offenhalten können.

## Ausführung und Nachweis

```sh
cd tests/blackbox
node --check door-hinge.mjs
node door-hinge.mjs --self-test
MC_SERVER_BIN=/path/to/frozen/ferrumc node door-hinge.mjs
```

Der Selftest prüft 16 eindeutige geschlossene/unpowered Eichentür-States und
die Ablehnung ungültiger Goldenwerte. Syntaxprüfung und Selftest bestanden.

Die unabhängige Quellenprüfung des Harness durch `select_world_ticket` endete
nach Ergänzung der echten Tragblockprüfung und Listener-Cleanup mit **CLEAN**.
Der anschließende Live-Lauf am 10.10.2026 bestand mit **Exitcode 0**:

- 14 echte Itemplatzierungen: vier Yaws × drei Cursorwerte und zwei Tür-
  platzierungen für den Mischmaterialfall.
- 28 erwartete Half-States per echten Blockupdates auf jedem der beiden
  verbundenen Clients, insgesamt 56 beobachtete Half-State-Prüfungen.
- Nach einem echten Prozessrestart 28 States aus unabhängig decodierten
  Chunks auf jedem der beiden frischen Clients, insgesamt weitere 56
  persistierte Half-State-Prüfungen.
- Zwei gestartete Serverprozesse; beide per SIGINT sauber beendet,
  Exitcode 0 und `shutdown complete` jeweils überprüft.
- Gefrorenes Binary:
  `.codex-tmp/wave01-bin/ferrumc`, SHA-256
  `364cd3b81aa114f0000ca217318b929c3cabf45f83d7202e3efb83d1e57f4878`.
  Derselbe Pfad und derselbe Hash vor beiden Starts und am Ende.

Lokale Nachweisartefakte:

- Clientlog: `/tmp/ferrumc-door-client-live.log`.
- JSON-Ergebnis: `/tmp/ferrumc-door-hinge-rfW1Sv/result.json`.
- Prozesslogs: `/tmp/ferrumc-door-hinge-rfW1Sv/server-initial.log` und
  `/tmp/ferrumc-door-hinge-rfW1Sv/server-restarted.log`.
- Isolierte World-DB: `/tmp/ferrumc-door-hinge-rfW1Sv/world`.

Diese lokalen Artefakte sind Arbeitsnachweise und keine eingecheckten
CI-Artefakte. Der Live-Test erzeugt bei jedem erneuten Lauf einen neuen Ordner
und erwartet weiterhin alle beschriebenen Beobachtungen.

## Grenzen

Diese Abnahme beweist die getesteten Placement-, Mehrspieler- und sauberen
Restart-Fälle innerhalb FerrumCs bisherigem Flatworld-Betrieb. Sie startet
keinen Paper-Prozess und belegt keine Paper-Differentialparität.
Horizontale angeklickte Faces, alle 21 Türmaterialien, Hindernisgewichte,
negative Koordinaten und obere Nachbartürhälften sind in der separaten reinen
Placement-Abnahme enthalten; dieser Clienttest erweitert diese Matrix nicht.
Shape-/Support-Regeln, Öffnen, Redstone, Gegenhälften-Abbau, allgemeine
Nachbarphysik, Plugin-Eventreihenfolge und Hard-Kill-Crashdurability bleiben
eigene offene Verträge. BLK-01 insgesamt und vollständiges Survival sind
hierdurch nicht abgeschlossen.
