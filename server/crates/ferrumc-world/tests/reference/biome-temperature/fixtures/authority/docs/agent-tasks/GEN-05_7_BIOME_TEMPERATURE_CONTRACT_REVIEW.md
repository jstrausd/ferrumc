# GEN-05.7 — Prüfung des Temperaturvertrags

Root hat den vollständigen305-Zeilen-Entwurf mit SHA-256
`be5b6f86ba846af690cf81f10b0b24b864796fadd44e63b0ce92c0906122d7a8`
gelesen. Ergebnis: **CLEAN auf Vertragsumfang** für den beschriebenen festen
Temperaturproducer; echte Oracle-/Runtimeabnahme bleiben erforderlich.

Der Werttyp besitzt65 überprüfte Biom-Klimadatensätze und fünf feste
Simplexobjekte. Compile/Clone/Query haben keinen Heap und die Query keinen
Worldseed, Cache oder RNGfortschritt. Die drei Originalnoises benutzen die
festen Legacyseeds1234/3456/2345 und offsets=false. F32-Konvertierungs- und
Operationsreihenfolge sowie sea+17 mit Java-i32-Wrap bleiben ausdrücklich Teil
der Abnahme. Paper public und beide Original-private Methoden werden geprüft;
Vanillas retained Publiccache ist separat diagnostisch.

Die positive Constructionallowance354536 ist ein deklarierter Admissioncensus
aus Quellbytes, Nodes,65 Records und1310 echten Original-RNGdraws. Sie ist
kein Instrumentierungs- oder CPUzyklusmaß. Alle statischen Ressourcen werden
vollständig syntaxgeprüft; der allocationfreie Decoder prüft escaped-key-
Duplikate mit festen Frames und Decodescratch. Actual Layout und Peak müssen
die vorgeschlagenen8KiB Retained-/1MiB Scratchcaps in der Runtime belegen.

Root hat alle64 Designfreezeinputs und661 tatsächlichen Inputs einschließlich
JDK frisch gehasht. Keine Abweichung. Designfreeze:
`f195f5c080335b9118083b60c3325f721b03123cd1f89f0d87f764b2f03de013`;
actual-inputs:
`01fa7a1aa4c0a4154b35f0121af8d6380195705e85f2eb537057f689a520142b`.
Der ursprüngliche input-only Vertrag `1323fbf0` bleibt unverändert.

Root gab zunächst nur neue genuine Originaloracle-Artefakte frei. Die erste
Autorserie fand keine stateless Temperaturabweichungen, meldete aber getrennte
Vanilla-Publiccache- und interne Paper-Offsetunterschiede sowie einen fehlenden
Frozen-innerfalse-Zweig. Diese vorläufigen Autorbefunde sind noch keine
unabhängige Root- oder Rustabnahme.

Vor einem weiteren Originalaufruf wurde deshalb eine separate, fest vorgegebene
Erweiterung genehmigt: frozen_ocean und deep_frozen_ocean, alle Integer-X/Z
von-64 bis63, Y79/Sea63. Alle32768 zusätzlichen Zeilen werden verglichen;
keine Auswahl einzelner Positionen anhand erwarteter Ergebnisse. Kombiniert
129588 Temperaturzeilen bleiben unter dem neuen expliziten Gesamtcap150000.
Die alten64 Designfreezeinputs bleiben immutable. Die vollständige finale
Normal/-O-Serie, Guardrefusals, tatsächliche Branchcoverage und neue Original-
Source-/Bytecodebelege werden vor der Worldproduktion unabhängig geprüft.

Worldruntime, Registry, Dependencies, GEN-05.6 und aktive App bleiben bis zu
diesem nachfolgenden Gate readonly. Der Producer aktiviert weder Surface,
Schnee/Eis, Naturwelt noch Survival und beweist keine TPS-/RSSverbesserung.
