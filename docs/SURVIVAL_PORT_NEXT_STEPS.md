# Survival: Fehlerkorrektur und nächste Portierungsschritte

Stand: 14.10.2026. Ziel bleibt Minecraft Java 1.21.8, Protokoll 772.
Der [Gesamtfeaturekatalog](MINECRAFT_FEATURE_CATALOG.md) bleibt der vollständige
Backlog; dieser Plan legt die nächste Ausführungsreihenfolge fest. Vorhandene
Subsysteme sind teilweise implementiert. Ein durchspielbares Spiel bis Nether,
End und Enderdrache ist weiterhin nicht vollständig abgenommen.

## Aktuelle Arbeitsaufteilung

| Arbeitsstrang | Tatsächlicher Agent | Änderung und Prüfung |
| --- | --- | --- |
| Session/Entity-Pakete | `session_backpressure` | Unveränderte Bewegungen und Metadaten unterdrücken; echte Bewegungen pro Viewer zusammenführen; Platz für Pflichtpakete in der vorhandenen FIFO reservieren; Spawn und Metadaten gemeinsam und begrenzt veröffentlichen. Laufversuch mit Gehen, Entity-Druck und Wiederbeitritt, danach Warteschlangen- und Sichtbarkeitstests. |
| Abbau und Pflanzen-Loot | `mining_loot` | Aktuelle Boden-/Wasserzustände im Tick, sofortiger Abbau bei Härte null, Scheren-/Schwertregeln und zufällige Seeds; vollständige zweihälftige Pflanzen vor dem ersten Write prüfen. Laufversuche mit Werkzeugzeiten, schnellen Folgeaktionen, echten Drops und Neustart, danach Owner-Regressionen. |
| Wasseraktivierung | `fast_chest_clicks` | Bestehende und neu geladene Wasserzellen sowie geladene Nachbargrenzen mit begrenzten fortsetzbaren Scans aufwecken. Echte Eimer-/Flow-/Neustartversuche ohne manuelles WaterSchedule, danach Grenz-/Queue-/Owner-Regressionen. |
| Integration und Speicherung | Root | Ein Folge-Abbau wartet auf die echte vorherige Werkzeugverschleißquittung. Tatsächlich beobachtete finale Vitals bleiben bis zum bestätigten Verbindungsabbau für den Inventar-Save verfügbar. Gemeinsame Binary, finale Prüfungen und Veröffentlichung. |

Der gemeldete Prozess lief nach der Trennung weiter. Das spätere
`UnknownPlayer` belegt die entfernte Session; die alten Zeilen benennen den
ursprünglich abgewiesenen Producer nicht eindeutig. Der gefundene Paketdruck
ist ein konkreter Codefehler. Neue Diagnostik nennt die ursprüngliche
Session-Ablehnung, Paket-ID und die separate Session-FIFO-Belegung.

## Reihenfolge nach dieser Korrektur

Die drei parallel nutzbaren Agentenplätze erhalten jeweils einen begrenzten
Crate-Auftrag. Root verbindet veröffentlichte Schnittstellen. Die folgenden
Pakete sind geplante Folgeaufträge, keine laufenden oder fertigen Funktionen.

| Priorität / Paket | Primärer Bereich | Konkretes Ergebnis und Abnahme |
| --- | --- | --- |
| 1: Gameplay unter Last | App, Session getrennt | Mehrere Spieler gehen und bauen bei laufender Generierung; langsame Leser, Teleports, Wiederbeitritte und Neustarts. Keine verlorenen Items oder Pflichtupdates; Logs unterscheiden Session-, Effekt- und Writer-Warteschlangen. Tickzeit und Chunk-Vorlauf werden unter diesem echten Workload gemessen. |
| 2: Vollständige Flüssigkeiten | Sim | Lava, Wasser/Lava-Mischungen, Obsidian/Kopfsteinpflaster/Basalt, Support und Wasserlogging aller vorgesehenen Blockfamilien. Ganze Writes und Drops vorab zulassen; natürliche sowie gespeicherte Flüssigkeiten über negative Chunkgrenzen und Neustart prüfen. Danach tatsächliche Schwimm-/Strömungs-/Ertrinkensconsumer. |
| 2: Licht und Chunk-Vorlauf | World, App getrennt | Reale Block-/Skylight-Grenzen nach Place/Break/Fluid, Tageslicht und Spawnprüfung; begrenzte Vorbereitung vor der Bewegungsrichtung. Lampen-/Höhlen-/Nachbarchunkfälle, laufendes Gehen und Speichern prüfen; Zeitmessung statt bloß höherer View-Distance. |
| 2: Items, Crafting und Container | Items, App getrennt | Versionsfeste Item-/Rezeptmatrix mit echten Consumern; restliche Click-/Drag-Arten, spezielle Rezepte, Rezeptbuch und Ofen-/Arbeitsblockinteraktionen. Schnelle Aktionen und zwei gleichzeitige Viewer, Komponenten und volle Inventare prüfen; jedes Item bleibt über Save/Restart erhalten. |
| 3: Vollständiges Mining und Loot | Items, Sim getrennt | Custom-Tool-Komponenten, Effizienz/Fortune/Silk Touch, Haste/Fatigue, Attribute und alle Block-Loot-Kontexte. Dauerhafte RNG-/Loot-Kontexte und XP-Orbs mit Pickup verbinden. Werkzeuge, falsche Tools, Enchants, Pflanzen/Crops und volle Drop-Kapazität mit Originalverträgen vergleichen. |
| 3: Kampf, Tod und Spielerphysik | Sim, App getrennt | Knockback, Schilde, kritische Treffer, vollständige Schadensarten/Effekte, Hunger, Bewegung/Posen, Tod/Respawn/Betten/Anker und Erfahrung. Originale Timer/Events und Multiplayer-Inventarerhaltung; Logout während Treffer, Food, Tod und Itemtransfer. |
| 3: Mobs und Navigation | Sim | Artenmatrix, Populations-/Spawn-/Despawnregeln, Pathfinding, Goals, Angriffe, Zucht/Zähmung und Loot. Aktuelle kleine Controller und Caps ersetzen keine vollständige KI. Reale Lichtprüfung, Chunk-Aktivierung, Navigation über Grenzen und echte Entity-Saves abnehmen. |
| 4: Overworld-Generierung | World | Originale Tree-/Foliage-/Root-/Decorator-Prozessoren und Feature-Reihenfolge, vollständige Erze/Vegetation und Strukturverwaltung. Danach Jigsaw/Templates, Dörfer, Minenschächte, Tempel, Strongholds, Monumente und Trial Chambers. Ganze Strukturen/Loot über Quellchunkgrenzen; feste Seeds gegen Originalergebnisse prüfen. |
| 4: Blockphysik und Verarbeitung | Sim | Nachbar-/Random-Ticks, Wachstum/Farmen, Schwerkraft/Feuer und weitere Block-Entities, Hopper/Dispenser/Brewing/Crafter. Alle automatisierten Transfers und Inputs vor Teilmutation prüfen; Save/Restart mit laufenden Prozessen. |
| 5: Dimensionen und Fortschritt | World, Sim getrennt | Nether-/End-Generierung, Portale, Dimensionsreisen, Bossmechanik und Fortschritt bis Dragon/Wither. Vollständige Survival-Strecke aus einer frischen Welt nach [Gesamtabnahme](SURVIVAL_END_TO_END_ACCEPTANCE.md). |
| 5: Redstone und weitere Entities | Sim | Strom/Nachbarticks, Kolben, Rails, Sensoren, TNT/Explosionen, Projektile, Fahrzeuge/Passagiere und dekorative Entities. Zeit-/Reihenfolgefälle sowie Speichern bei aktiven Maschinen abnehmen. |
| Parallel, nach Consumer-Verträgen: Java/Paper | Plugin/Bridge, Sim getrennt | Lifecycle, Rust-Tick-Scheduler, APIs, Events und Cancellation/Reentrancy für die jeweils realen Rust-Consumer. Externe unveränderte Plugin-JARs mit gepinnten Funktionen prüfen; fehlende APIs bleiben ausdrücklich offen. |
| Durchgehend: Daten und Betrieb | Storage, Net, Tools getrennt | Auth/Profile, Datapacks/Tags/Loot/Recipes, Commands, Backup/Recovery, dauerhafte Transfers und Betrieb unter Last. Keine Registry-ID ohne Consumer; Ressourcenvergleich mit Paper bei vergleichbarem Gameplayumfang. |

Für jede Vergabe: vollständige Originalmethode einschließlich Callees und
relevanter Paper-Source-/Feature-Patches lesen, Eigentümer und Rückgaben
festlegen, implementieren, privat ausprobieren, **anschließend** gezielte
Regressionen ergänzen und den integrierten Stand prüfen. Breite Wiederholungen
bereits bestandener fremder Suites sind kein Ersatz für eine konkrete Abnahme.

## Referenz und bestehende Grenzen

Die Referenz ist Paper 1.21.8 Build 60, Source-Commit
`29c8822d90899c89d2689338e81a98f690bcba12`; lokal liegen die vollständig
angewandten dekompilierten Originalquellen und Patches vor. Details und
Quellenvertrag: [PAPER_REFERENCE.md](PAPER_REFERENCE.md). Die Umsetzung folgt
den dokumentierten Verhaltensverträgen. Eine Quellenlektüre oder eine bestandene
FerrumC-Probe ist noch kein bestandener Paper-Differentiallauf.

`./start.sh` verwendet für neue Welten bereits das aktuell unterstützte
`ferrumc_natural_playable` und standardmäßig Debug-Logging. Revision 5 enthält
Bäume, Vegetation, die implementierten Erze sowie Wüstenbrunnen-, Hütten- und
Cabin-Strukturen; [genauer Umfang](PLAYABLE_WORLD_GENERATION.md). Alte Saves
behalten ihre Generatoridentität. Dieser Plan verlangt weitere originale
Strukturen und Features und erklärt den vorhandenen Teilumfang nicht zur
fertigen Vanilla-Weltgenerierung.

Persönliche Server, Konfigurationen und gespeicherte Welten werden für diese
Entwicklungsversuche nicht gestartet oder verändert. Die Laufversuche verwenden
eigene lokale Ports und Save-Verzeichnisse.
