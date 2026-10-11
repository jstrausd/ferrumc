# SIM-03.4 — unabhängige Vertragsprüfung

Root hat den vollständigen konkreten Publicationentwurf mit SHA-256
`efbeb2beb26198f6b6d5ae3904388fa91cfe8f91ced42f44ece012549df986fa`
gelesen. Ein separater readonly Reviewer hat ihn gegen die tatsächlichen
Admission-, Completion-, Countershadow-, Release-, Restore- und Stopabläufe
geprüft. Ergebnis: **CLEAN auf Vertragsumfang**, keine blockierenden Findings.
Runtime, Sessionviewer, Collectortracking und Survival sind damit noch nicht
abgenommen.

Die gewählte Form reserviert Gruppenzahl, immutable Bodycharge und checked
Sequencecredit vor Workrequest. Confirm verschiebt diesen Credit vollständig
in eine retained FIFO; nur definitive Abortfreigabe oder gültiger Headack
gibt ihn frei. Der immutable Scalarcutpoint liegt nach echter
Counterconfirmation und vor späterem Counterboundary. Kopien benötigen keine
zweite Mutationsauthority. Bounded bestehende Item-/NBT-Deepclones bleiben
ausdrücklich infallible Allocatorgrenzen; neue Outerqueues sind fallible.

Ack und peek bleiben während Drain verfügbar. Stop und Schedulerquiescence
verlangen leere reserved/retained Ledger. Der readonly Countershadow verwendet
denselben Publicationpredicate und führt die echte Completionprefix-/FIFOfolge:
Confirm hält eine retained Gruppe, definitive Abortfreigabe gibt nur ihre
Reservation frei, neue Restore-/Actiongates liegen vor weiteren
Shadowresourceänderungen. Das muss die Runtimeabnahme tatsächlich prüfen.

Die immutable Collectorbindung wird aus der wirklichen validierten
Pickupadmission captured. Rejoin darf sie nicht nachträglich aktualisieren.
Die tatsächliche Session-/Wire-/Audiencebindung benötigt einen separaten
Sessionproducer; Original-Take verwendet Collectortracking.

Root hat die sechs finalen Freezeinputs,48 ganzen Simdateien,30 eingefrorenen
Metadatafiles,21 Originalsource-/Patchpins und15 übrigen Producerpins frisch
gehasht. Keine Abweichung. Finaler Designfreeze:
`fc39831556a5b8f8f11ddc4b7e05af03592e66fb70c50829037804026382e92a`.
Der frühere Zwischenfreeze bleibt unverändert. Separater Review unter
`.codex-tmp/sim03_4-independent-contract-review/REVIEW.md` und `assessment.json`.
Keine neue Originalexportserie, kein Gameplayvergleich und keine Runtimechecks
wurden durch diesen Vertragsreview behauptet.

Root gibt ausschließlich `ferrumc-sim`, den eigenen SIM-03.4-Brief und
Scratchevidenz für die beschriebene Produktion frei. Nach der Implementierung
folgen whole-crate Debug/Release, strikte Clippy-/Fmt-/Rustdocchecks,
meaningful Memory-/redb-/Credit-/Cutpoint-/Retry-/Shadowtests und unabhängiger
Rootreview vor Integration. Session, App, Storage, Items, NBT, Registry,
Dependencies und Checkpointformat bleiben außerhalb dieser Vergabe.
