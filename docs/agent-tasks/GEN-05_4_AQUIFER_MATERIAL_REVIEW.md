# GEN-05.4 — unabhängiger Rootreview

Entscheidung: **CLEAN** für den begrenzten World-Materialproducer.
Autor: `item_transfer_primitives`; unabhängiger Reviewer/Integrator: Root.
Referenz: Minecraft1.21.8/Protokoll772/DataVersion4440, PaperBuild60,
Commit `29c8822d90899c89d2689338e81a98f690bcba12`.

Der Producer wählt tatsächliche Blockstates und Flüssigkeitsflags aus den
frischen NoiseChunk-Dichten mit Aquifer- und Erzaderlogik. Material- und
Densitylauf teilen denselben konkreten Driver; Sessionauthority, Poison und
atomarer Reset erhalten ihre bereits geprüften Grenzen. Die Materialausgabe
besitzt ihre States und getrennte Fluidbitmap selbst. Metadatenauflösung und
Aquifercaches sind begrenzt; der Blockloop erzeugt keine neuen Vektoren.
Zusätzliche Input-/Tracehooks sind ausschließlich `cfg(test)`.

## Unabhängige Prüfung

Root hat alle **160 World-/Briefdateien** nach Größe/SHA256 geprüft und in
ein frisches Archiv auf Basis `f6eb7111ad11569fba51209f74f24b9321d0c66c`
übernommen. Sim-/Proto-Agentänderungen waren darin ausgeschlossen. Alle fünf
Autorlogs und die zugehörigen Evidencepins wurden unabhängig nachgerechnet.
**109/118** Dateien des vorherigen Worldfreezes bleiben bytegleich; exakt
die neun zuvor erklärten Runtime-/Testhook-/Dokudateien sind verändert.
Alle alten 24 Fixtures und 28 Referencefiles bleiben unverändert.

Die elf übernommenen Originalhelper-/Input-/Outputdateien sind bytegleich
zu den unabhängig erzeugten Rootreferenzen. Die beiden Originalmethoden-
Exporte wurden von Root zuvor frisch reproduziert, einschließlich Origins,
Manifesten und jeweils fünf Helperclasses. Diese bestehende unabhängige
Evidenz wurde übernommen; es wurde kein Rustalgorithmus als Javaoracle benutzt.

Im frischen Archiv bestanden:

- WholeWorld Debug und Release jeweils **205 Unit +46 Integration =251**
  Tests, ohne Fehler oder ignorierte Fälle.
- Striktes Alltargetclippy mit `-D warnings`, Formatcheck und Rustdoc mit
  `RUSTDOCFLAGS=-D warnings`, ohne Lintausnahmen.
- **126 vollständige Chunks /7.077.888 Blockstates und Flüssigkeitsflags**
  gegen die gepinnten tatsächlichen Paperarrays.
- **499+42 finite Originalmethodenfälle**, einschließlich Rawbits,
  Density-/RNG-/Contextchronologie, vollständigen Cachezuständen, strikten
  Ties, Earlyreturns und genauem N/N−1-Workbudget.
- **15 Nonfinitekontrollen** als ausdrücklich zusätzliche Rustadmission,
  getrennt vom tatsächlichen Javaresultat; Halo-/Scaled-/Packeddomains,
  Vorbereitung, getrennte Writes, Poison/Reset und Ergebnisbesitz.

Nach allen Werkzeugläufen waren sämtliche 160 Autorenpins sowohl im Archiv
als auch im Arbeitscheckout unverändert. Der direkte Codereview umfasst
öffentliche Ausgabe-/Lifecyclemethoden, Materialdriver, Stateauflösung,
Aquiferstatus-/Pressure-/Surface-/Spread-/Lava-/Rankingpfade und metered RNG.
Keine weitere Produktionskorrektur war erforderlich.

Autorfreeze: `.codex-tmp/gen05_4-author-world.freeze.json`, SHA256
`be33837c431d0cbc57f0e89cc3c26d054e556e15490190ccbb332ab321c27763`.
Unabhängiger Lauf: `.codex-tmp/gen05_4-root-review/`; Commands, Exitcodes und
Logpins in `checks.json`, Rootrunner `.codex-tmp/gen05_4-root-review.py`.

## Geltender Umfang und nächste Verbraucher

Dies ist die frische Materialausgabe **vor Surface, Carvers, Features und
Strukturen**. Sie ist noch kein aktivierter natürlicher Weltgenerator.
Die Pointleafreferenzen belegen ihre tatsächlich benutzten Computepfade;
ihre unbenutzten min/max-Metadaten sind kein vollständiger DensityFunction-
Vertrag. NaN/Inf verweigert Rust nach benannter Policy, während Java dafür
auch Resultate liefert. Diese Unterschiede werden nicht als Parität ausgegeben.

REG-BIOME-01 liefert als nächste separate Registryarbeit vollständige
Biomekeys/-Definitionen und geordnete Originalparameter. Danach folgen der
historyhaltige RTree-Selector/Quartarrays, Surface/Bedrock und ausdrücklich
getrennte Geometry-/Chunk-/Storage-/Sim-/Appconsumer. Flat-v1 bleibt aktiv;
Survival S1–S6, Nether/End/Dragon, Java-/Paperplugins und RAM-/TPSverbesserungen
sind weiterhin offen.
