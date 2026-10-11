# DUR-04.2a — unabhängige Counter- und Post-fixabnahme

Status: **CLEAN** für den begrenzten Simconsumer. Autor
`select_world_ticket`, Reviewer Root. Das ältere Pre-fixassessment bleibt
unverändert **CHANGES_REQUIRED**: Größen- und Repräsentationsproben hatten
vollständige temporäre FCIS-Buffer erzeugt. Dieser Befund wurde vor Integration
durch den separat geprüften ITEM-SIZE-01-Producer und seine enge Adoption behoben.

Root hat den Counter-/Ownership-/Boundarycode und die gesamte Korrektur gelesen.
Sieben reine Größenstellen verwenden jetzt `persisted_stack_size`; ein Vergleich
verwendet `same_stack_representation` nach dem unveränderten privaten
Empty-/Itemguard. Der endgültige Korrekturdiff betrifft ausschließlich vier
Runtimefiles, INVARIANTS und den Aufgabenbrief. Tatsächliche FCIE-/FCPS-
Materialisierung und Persistenceencodes bleiben erhalten. Legacychest ist kein
Consumer dieses Tickets.

Die Reihenfolge bleibt Completionprefix, Defer, Aktionen, Counter, Save. Alle
residenten Items erhalten pro angenommenem logicalTick ihren skalaren Schritt;
die getrennte Saveauswahl ist auf64 begrenzt. Confirmed-before/live-after und
reservierte Bodybytes bleiben getrennt. Terminale Generation verweigert vor
Scratch/Drain; Save-IDs kommen aus dem32-Slot-Pool. Die dokumentierten9MiB
Scalarscratch-,1024-Defer- und16MiB-Live-/Confirmedgrenzen sind lokale Caps.

Root hat einen frischen `b55513957d7277c1d79544b7ae8d7b4952d3e703`-Archivstand
mit ausschließlich48 Simoverlaydateien geprüft. Debug und Release bestehen
jeweils320 Tests; striktes Alltargetclippy, Workspacefmt und Simrustdoc mit
`-D warnings` bestehen ohne Ausnahmen. Die vorhandenen30 Countertests decken
die tatsächlichen Boundary-,Busy-,Save-,Lostreply-/Reopen- und Refusalpfade ab.
Für die semantikerhaltende APIadoption waren keine neuen Spiegeltests nötig.
Alle48 Sim-,28 Items-,22 Paperreferenz- und vier historischen Artefaktpins
sind vor/nach der unabhängigen Prüfung gleich; der finale49-Datei-Sourcefreeze
enthält exakt den geprüften Simstand plus seinen Brief.

Finale Sourcefreeze `.codex-tmp/dur04_2-postfix-final/source.freeze.json`:
`8670b36f94c839f5cb89271cb3903f03376a5ca7f0a4f81516a570688ce2a975`.
Evidencefreeze: `a4554a027298b8316ae830595c41ebee3d042fe22576ae1cae724d56e7e9ec52`.
Unabhängige Commands, Exitcodes und Loghashes stehen unter
`.codex-tmp/dur04_2-postfix-root-review/checks.json`.

App-Format6adoption, Physik, Merge, automatische Aufnahme, Sessionpublication,
Javaevents und die vollständigen Survivalabnahmen bleiben eigene offene
Consumer. Diese Abnahme ist kein gestarteter Paper-Gameplayvergleich und keine
RSS-/TPSmessung.
