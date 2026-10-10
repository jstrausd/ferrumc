# DUR-04.1 — unabhängige Abnahme des Player-/Itemconsumers

Ergebnis: **CLEAN für den beschriebenen opt-in Simconsumer**. Autor:
`select_world_ticket`; unabhängiger Review und Integration: Root.
Die produktive App hält weiterhin ihr bisheriges Inventar und startet Format4.
Dieses Ticket aktiviert weder Online-Drop/Pickup noch vollständiges Survival.

## Unabhängig geprüfter Stand

Root hat einen frischen `git archive` von `e563b97171b8b812b8c866854ce09cdcf45c8e64`
erzeugt und ausschließlich die43 eingefrorenen Simdateien überlagert.
Der Autorenmanifest SHA256 ist
`de0b202ab2fe52246758bddc1f8ab3ac7b0d8ff7c5f6c26201786e640fc94aca`,
das Runtimearchiv
`6bc4cae0a1ede74ae4955ad187744e67139393655089595ad2f1d84614e6979e`.
Alle43 Runtimepins,22 vollständigen Basis-/Appliedreferenzen und drei
Appfixtureinputs wurden unabhängig nachgerechnet. Der finale44-Dateien-
Ownedfreeze einschließlich Autorenbrief hat SHA256
`fb5b20db151d3aa937bffdc2a889f00e6cad2ded51e52ca663d447fc4d995907`;
alle fünf finalen Autorenlogs stimmen mit dem Ergebnismanifest überein.

Die Referenz ist Minecraft Java1.21.8/protocol772, Paper Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Quellenprüfung umfasst vollständige
Drop-/Pickup-/Inventory-/ItemStack-/Packetmethoden und betroffene Patches,
nicht allein Interfaces. Eine laufende Paper-Spielabnahme folgt separat.

Root prüfte besonders die Storebindung, versiegelten Snapshot-/Receipt-/
Currentpostimagebelege, konkrete Pendingreservation, bestätigte gemeinsame
Publikation, Entitywriter-Gates und den SnapshotOnly-Clonepfad. Ein Receipt
mit historischen Afterstamps legitimiert keinen späteren Currentstate. Fehlende
Receipts beweisen keinen Abbruch eines noch laufenden Storageworkers.

## Eigene PublicAPIproben und Checks

Die drei zusätzlichen Proben in
`server/crates/ferrumc-sim/tests/root_player_contract.rs` laufen jeweils mit
dem tatsächlichen Memory- und redb-Ownershipbackend:

1. Nach Verlust eines Bootstrapworkbuffers publiziert ein readonly Receipt-
   `None` keinen Player. Pending und Reservation bleiben erhalten;
   unbewiesener Workerabbruch verweigert Retry und Stop ausdrücklich.
2. Während eines reservierten Drops angenommene Hotbarauswahl und Bewegung
   erscheinen im nächsten Drop und dessen dauerhaftem vollständigem FCPS-
   Postimage. Player und zwei Item-Entities erhalten zusammen alle14 Items.
3. Zwei gleichzeitig vorbereitete Pickups desselben Items publizieren keine
   konkurrierenden Mengen. Der erste Collector erhält zwei von sechs Items;
   der zweite benötigt einen frischen Pairread und erhält danach die vier
   übrigen Items. Fullpickup entfernt die Entity mit dauerhaftem Tombstone.

Der frische Archivlauf besteht Whole-Sim Debug und Release mit jeweils
**290 Tests**:174 Unit,111 Integration einschließlich der drei Rootproben,
fünf Doctests. Strict All-target-Clippy, Fmt und Rustdoc mit `-D warnings`
bestehen ohne Lintausnahmen. Die ursprünglichen43 Runtimehashes bleiben nach
Formatierung und sämtlichen Checks unverändert. Der integrierte Roottest ist
die tatsächlich geprüfte und formatierte Archivdatei.

Die Autorenmatrix enthält außerdem echte Drop1/All, Partial/Fullpickup,
Slot-/Offhandreihenfolge, Komponenten-/Mengenbilanz, normale Links-/Rechts-
Klicks, CAS-/ABA-/Reopen-/Busy-/Lostreply-/Currentreadfehler, supersedete
Receipts, fremde Store-/Owner-/Handleinputs, Kapazitätsgrenzen und
Disconnect-/Rejoin-/Unload-/Drain-/Clonefälle. Parser verwenden ein unabhängig
gerahmtes Appformatfixture und feste Byteoffsets sowie Fehlformgrenzen.

## Belegter Vertrag und verbleibende Verbraucher

Der Consumer besitzt46 Slots und einen Cursor unter einer tatsächlichen
Shardauthority. Vollständige FCPS3-/FCIS-Daten, konkrete Drop-/Pickuppläne,
Playerreservation und maximal eine Entityoperation pro Shard bleiben bis zu
echtem passendem Receipt und Currentpostimagebeleg gebunden. Count-/encoded-
Bytecaps gelten vor konkreten Decode-/Clone-/Requestpfaden; sie sind keine
Zusage über vollständigen Prozess-RSS. Beliebige Backendfremdwriter nach
Evidencecapture werden nicht durch eine verteilte Storagelease verhindert.
DUR-05 muss den einzigen produktiven Writerdriver pro Scope/UUID sicherstellen.

Dropdelay40 hat in `.1` noch keinen ownerlokalen Timerconsumer. DUR-04.2 muss
bestätigten Entityrecord und veränderlichen Livestate getrennt behandeln,
begrenzte coalescte Saves und Pending-/Due-/Unloadchronologie liefern.
Physik, Aktivierungsregeln, Merge, Despawn, Tracking und Javaevents folgen
als konkrete Verbraucher. Zwei einzelne Entitysaves ersetzen keinen atomaren
Entity↔Entity-Merge.

DUR-05 benötigt explizite Format6-/Memoryadmission und atomaren Übergang der
heutigen Connectioninventare. Format4/5-Bestände benötigen ihre eigene
explizite Migration mit deklarierter Player-Scopezuordnung. Mining/Loot,
Onlinecrafting, Kampf/Mobs, Nether/End/Dragon und sämtliche offenen
Survival-Gesamtabnahmen bleiben erforderlich.

Rootbelege liegen unter `.codex-tmp/dur04_1-root-review/`. Der10-Dateien-
Reviewfreeze hat SHA256
`81e12071b3020ca70a26a7dc8376f76665ac5913b63634070b8689f680129822`.
Die GitHub-CI des neuen Integrationsheads wird separat ausgeführt;
die grüne CI von `e563b971` gilt für dessen vorherigen Runtimeumfang.
