# NET-04.1b — unabhängiger Netconsumerreview

Ergebnis: **CLEAN für den begrenzten Netconsumer**. Root hat eine frische
Archivkopie von `c3d75914` erzeugt und ausschließlich den eingefrorenen
Net-/Briefscope übernommen. Fremde Sim-/Worldarbeit war nicht enthalten.
Der separate NET-04.1-Protoproducer bleibt unverändert; beide Cratecommits
werden gemeinsam veröffentlicht.

## Verhalten und unabhängige Prüfung

SetEntityData, SetEntityMotion und TakeItemEntity sind in beiden exhaustiven
Matches ausdrücklich State/Mandatory. Mit vorhandenen SpawnEntity und
RemoveEntities teilen sie bei erfolgreicher Defaultadmission dieselbe FIFO.
Motion bleibt eine konservative lokale Zuverlässigkeitspolicy, bis ein echter
Movementconsumer Supersession oder Healing beweist.

Root hat die elf neuen Tests und den vollständigen Produktionsdiff gelesen.
Die Tests verwenden reale PlayWriterqueues, Frame-/Clientbounddecoding und
Compression. Sie prüfen Spawn/Data/Motion/Stop, Partialrest/ursprüngliche
Takecounts, gemischte Prioritäten, Frame-/Bytebatchgrenzen, den maximalen
263168-Byte-Rawtail, volle/Null-Statequeues und zurückgegebenen Refusal ohne
Verlust des Backlogs. Zwei negative reale Overrides belegen Metadata vor
Spawn bzw. Remove vor Take; der enge Framecap hält den verweigerten Frontpacket.
Opaque Metadata und signed Wirecounts werden nicht normalisiert.

Writerbytes außerhalb Rustdoc sind exakt baselinegleich. Root hat die
tatsächlichen readonly Appcaller `queue_outbound` und `is_mandatory_overflow`
mitgeprüft: Der Writer besitzt keine Criticality und gibt Dropped zurück;
der Caller eskaliert Mandatoryrefusal. Nettests belegen diesen Refusal,
keinen Appdisconnect. FIFO bedeutet keine atomare Bundleadmission.

## Frische Reviewergebnisse und Pins

Rootlauf `.codex-tmp/net04_1b-root-review.py`; Ergebnisdatei und Logs unter
`.codex-tmp/net04_1b-root-review/`:

- Debug und Release jeweils196 Tests, keine Fehler oder ignorierten Fälle.
- Strict Alltargetclippy, isoliertes Workspacefmt und Net-Rustdoc
  `--no-deps` mit `-D warnings` bestanden, keine Lintsuppression.
-36 Source-/15 Evidence-/79 unveränderte NET-04.1-Dateien sowie acht tatsächliche
  Paper-Sendepfadsourcepins vor und nach den Checks unabhängig verifiziert.
- Die Archivoverlaybytes blieben nach allen Checks identisch.

Sourcefreeze `.codex-tmp/net04_1b/source.freeze.json`, SHA256
`b63e7c9830db610350db673f2a237c0e7bbd603fb5935752ed028734ad73411b`.
Evidencefreeze `.codex-tmp/net04_1b/evidence.freeze.json`, SHA256
`bdfb50feadb3fd84f8c5ea7df75302494baa86347c8782bbe1e3f4d431ad32fd`.
Die bestehende echte NET-04.1-Paper-/Prismarinefixture und ihre Semantikgrenzen
werden unverändert übernommen; kein redundanter Originalserverboot.

Die Autorenprüfung fand vier bestehende öffentliche Net-Rustdoclinks auf
private Module. Sie wurden als reine Docverweise korrigiert, ohne Sichtbarkeit
oder Runtime zu ändern; die frische Rootprüfung bestätigt strikte Netdocs.
Eine zusätzliche Dokumentation aller Dependencies meldete den unveränderten
Codec-Privatlink `blob.rs:41`; dieser zusätzliche Befund bleibt separat offen
und wurde weder durch eine Codecänderung noch eine Lintausnahme verdeckt.

## Genaue Grenze

Keine Sessionviewer-, Itemmetadata-, Sim-/Storage-/Appaktivierung. Keine
Bundleatomicität, Motionphysik, automatische Pickupentscheidung, Mobs,
Javaevents oder Survivalabnahme. NET-04.2a/.2b und DUR-05 besitzen diese
anschließenden Consumer. Gesamt-CI wird für den tatsächlich veröffentlichten
gemeinsamen Head geprüft; ein älterer Lauf ersetzt diesen Nachweis nicht.
