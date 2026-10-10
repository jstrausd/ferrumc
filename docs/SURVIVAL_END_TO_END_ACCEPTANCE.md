# Durchspielbares Survival bis zum Enderdrachen

Zielversion: Minecraft Java Edition **1.21.8, Protokoll 772, DataVersion 4440**.
Diese Abnahme ergänzt den [214-Ticket-Katalog](MINECRAFT_FEATURE_CATALOG.md)
und das [Implementierungsboard](IMPLEMENTATION_BOARD.md). Sie ersetzt weder
die vollständige Inhaltsmatrix noch Plugin-, Betriebs- oder Performanceabnahme.
Stand der Vergabe: `347080f4`; **alle Etappen sind offen**.

## Fortschrittsstrecke

| Etappe | Tatsächlich erforderliches Gameplay | Wesentliche Abhängigkeiten |
| --- | --- | --- |
| S1: neue Welt und erster Tag | Natürliche, gespeicherte Overworld mit sicherem Spawn; Bewegung/Kollision; Holz und Stein korrekt abbauen, aufnehmen, Werkzeuge und Werkbank craften; Tag/Nacht, Licht, Nahrung und Bett. | GEN-01–07/11–12, SIM-02–11/13, DUR-01–05/08, SRV-01–08/14/16, BLK-01/06–12/14 |
| S2: Ausrüstung und Überleben | Höhlen, Erze, Schmelzen, Brennstoffe und Haltbarkeit; Hunger, Regeneration, Schaden, Rüstung, Schild, Fernkampf, XP und Tod/Respawn; natürliche passive und feindliche Mobs mit Loot. | SRV-03–15/17–21, MOB-01–06/14–16, BLK-03–07/15, SIM-03/05/07/09 |
| S3: Netherfortschritt | Obsidian herstellen/abbauen, Netherportal bauen und entzünden; zuverlässige Hin-/Rückreise, Netherterrain/Biome/Strukturen, Blaze/Blazerods und Enderperlen, Brauen und Schutz gegen Umweltgefahren. | GEN-08/10, MOB-07, SRV-04/09/15/18, BLK-03–06/18, DUR-04–08 |
| S4: Zugang zum End | Enderaugen craften und verwenden, echte Strongholdgenerierung und Locateverhalten, vollständiger Endportalrahmen, Aktivierung und Dimensionswechsel; Endterrain und Spawnplattform. | GEN-07/09–10, SRV-05/19/20, MOB-08, BLK-21 |
| S5: Enderdrache | Dragonphasen/Flug/Kollision/Angriffe, Kristalle und Heilung, Schaden an richtigen Teilentities, Bossbar, Kampf und Tod; XP, Exitportal, Drachenei, Abspann und Rückkehr. | MOB-14–16, SRV-09–15/22–23, GEN-09–10, DAT-04/06 |
| S6: danach weiter spielen | Endgateway, äußere Inseln, Endcities/Shulker, Elytra; Dragonrespawn, Wither und weiterer Fortschritt; Farmen, Villager/Handel/Raids, Redstone, Transport und alle übrigen Inhaltsfamilien. | GEN-07/09–10, MOB-08–16, SRV-17–23, BLK-01–23, DAT/CMD-Matrix |

Eine kurze Strecke zum ersten Dragonkill genügt nicht zur Freigabe des gesamten
Minecraft-Umfangs. Auch S6 und die übrigen Katalogzeilen müssen den jeweiligen
Familien-/Arten-/API-Vertrag erfüllen. Hilfstypen, Registrys oder reine
Matcher gelten erst mit aktiviertem, geprüftem Gameplayconsumer als Fortschritt
einer Spielabnahme.

## Reproduzierbarer Lauf und Abnahmeevidenz

Jeder Lauf bindet Gitcommit, Serverbinary-SHA, Minecraft-/Paper-Pin, Seed,
Konfiguration, Schwierigkeit, Gamerules und Pluginmatrix. Ein Lauf aus einer
frischen natürlichen Welt verwendet Survival ohne Operatorrechte; Items,
Portale, Mobs und Fortschritt entstehen durch ihre regulären Spielmechaniken.
Vorbereitete isolierte Fixtures prüfen zusätzlich seltene und fehlerhafte
Fälle, ersetzen diesen Fortschrittslauf jedoch nicht.

Zwei echte Vanilla-Clients prüfen Spieler- und Beobachterzustand. Unabhängige
Protokollharnesses zeichnen die reproduzierbaren Aktionen und autoritativen
Ergebnisse auf. Referenzläufe auf dem gepinnten Paper prüfen denselben
Mechanikvertrag mit denselben Parametern; Quellenlektüre allein schließt diesen
Nachweis nicht. GUI-/Bewegungs-/Kampfverhalten erhält echte Clientabnahme.

Jede Etappe enthält sauberen Stop/Neustart und Wiederbeitritt. Wiederöffnete
Welten erhalten Terrain/Seed, Block-/Containerzustand, Items und Komponenten,
Entityidentitäten, Spielerzustand und erreichten Fortschritt. Dimensionswechsel,
Chunk-Unload, Tod und Disconnect werden auf ihre dauerhafte Ownership geprüft.
Crash-/Lost-Reply-/Diskfehler verlangen separate reproduzierbare Cutpoints;
ein SIGINT-Neustart ist kein Nachweis gegen einen Prozessabbruch.

Vorhandene und neu erzeugte Items werden je Transferquelle/-ziel bilanziert.
Zwei konkurrierende Pickups, volle Tasche, Teilaufnahme, Containerwechsel und
abgebrochene Aktionen dürfen weder Duplikate noch unbeabsichtigten Verlust
erzeugen. Erzeugung und Verbrauch durch Loot/Crafting/Tod folgen dokumentierten
Regeln und der tatsächlichen Eventreihenfolge.

## Abschluss und Ressourcenmessung

Freigabe verlangt offene Fallzahl **null** in der deklarierten Inhalts- und
Pluginmatrix, alle sechs Etappen, Fehler-/Recoveryabnahme und veröffentlichte
CI für genau den freigegebenen Commit. Jede bewusste Abweichung bleibt sichtbar
und verhindert eine weiter gefasste Drop-in-/Kompatibilitätsbehauptung.

RAM, CPU und Tickzeit werden einschließlich Java-Brücke mit Paper bei gleichem
Terrain-, Simulations-, Mob-, Spieler- und Pluginumfang gemessen. Die
Programmiersprache allein ist kein Performancebeleg. Die vorhandenen
Performanceverträge und Lastszenarien gelten weiter; Zahlen werden erst nach
vergleichbaren Messungen als Verbesserung ausgewiesen.
