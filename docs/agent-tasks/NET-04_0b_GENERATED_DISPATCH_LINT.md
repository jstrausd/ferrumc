# NET-04.0b — lokale Zeilengrenze generierter Decode-Dispatchtabellen

Status: Entwurf von Root freigegeben, Implementierung und begrenzte lokale
Prüfungen abgeschlossen. Exakter Freeze vor unabhängiger Rootabnahme;
noch kein CLEAN oder Produktionsadoption.
Autor `/root/feature_catalog` besitzt ausschließlich
`server/crates/ferrumc-proto-gen/**`, diesen Brief und eigenen Scratch
`.codex-tmp/net04_0b/`. Produktionsspec, generierte Produktionsdateien,
Runtimecrates, Cargo-/Lockfiles und Git bleiben read-only.

## Konkretes Problem

Das Hinzufügen der drei für NET-04.1 nötigen Packetarme überschreitet nach
echter rustfmt-Normalisierung die bestehende100-Zeilen-Clippygrenze der
ClientboundPlayPacket-Decodefunktion: aktueller Body96, mit SetEntityData,
SetEntityMotion und TakeItemEntity101. Ein read-only Clone der tatsächlichen
Generatormethode wurde mit clippy-driver gegen minimale Stubs geprüft:
Baseline grün, Drei-Packet-Ausgabe wegen clippy::too_many_lines verweigert.
Belege liegen unverändert unter `.codex-tmp/net04_0/dispatch-size/`.

## Begrenzter Entwurf

Nur `src/emit.rs::render_dispatch` ergänzt unmittelbar vor der erzeugten
`decode(id, reader)`-Methode ein methodenspezifisches Attribut:

```rust
#[allow(
    clippy::too_many_lines,
    reason = "Generated dispatch tables keep one arm per packet in stable schema order."
)]
```

Dieses Attribut gilt für jede nichtleere erzeugte State-/Direction-
Decode-Dispatchtabelle. Es gilt nicht für Packet-/Structdecoder, Constructoren,
Getter, packet_id, Encoder, Module, Crates oder handgeschriebene Funktionen.
Es verändert keinen Dispatcharm, keine unbekannte-ID-Verweigerung, keinen
Wiretyp und keine API. Der Generator selbst erhält keine Lintausnahme.
Die README dokumentiert den tatsächlichen engen Scope. Keine neuen
Dependencies, Testschemata in Produktion oder lintweite Projektkonfiguration.

## Nachweis und getrennte Adoption

Bestehende Whole-Generatortests, striktes Alltarget-Clippy/Fmt/Rustdoc.
Im eigenen frischen git-archive überlagert der Autor ausschließlich den
Generator und führt den echten xtask mit einem separaten Target aus.
Die aktuelle Produktionsspec wird in dieser Kopie neu erzeugt; entfernt man
nur das neue normalisierte Attribut, bleibt jeder bisherige erzeugte Inhalt
bytegleich zur archivierten Baseline. Wiederholte Generation und --check
sind bytegleich/grün.

Nur im Archiv ergänzt ein getrenntes Testschema die drei echten Packetnamen
und IDs/Feldshapes von NET-04.1. Diese durch den echten Generator erzeugte
Proto-Runtime muss unter dem unveränderten strikten Alltarget-Clippy grün
sein. Die drei Shapes dienen hier nur als Größen-/Generierungsprobe;
originale Wirefixtures und APIabnahme bleiben NET-04.1.

Exakter Source-/Evidencefreeze vor Rootreview. Die Generatoränderung erzeugt
absichtlich Drift gegenüber den read-only Produktionsdateien. Root adoptiert
diese separat mit `cargo xtask generate` als NET-04.0c und integriert/pusht
beide Änderungen gemeinsam; kein alleiniger Push mit Produktionsdrift.
Erst nach Rootintegration beginnt die tatsächliche NET-04.1-Spec-/Runtimearbeit.

## Implementierung und abgegrenzte Ergebnisse

`render_dispatch` ergänzt ausschließlich das freigegebene lokale Attribut;
README dokumentiert dessen Scope. Keine Parser-, API- oder Wireänderung.
Der finale isolierte Stand ist ein frisches `git archive` von
`4f7d50371a8a12560a9d93957a70ac009dc6b9bb`, mit nur `src/emit.rs` und
`README.md` des Generators überlagert. Eigener separater Cargo-Target;
keine gemeinsame xtask-Binary mit der Produktion.

- Generator: 27 Unit- plus2 Whole-Generator-Tests, striktes Alltarget-Clippy,
  Fmt und uneingeschränktes Rustdoc mit `-D warnings` grün.
- Tatsächlicher xtask gegen unveränderte archivierte Produktionsspec:
  fünf geänderte State-Dateien, genau neun lokale Decode-Dispatchattribute.
  Nach Entfernung ausschließlich dieser normalisierten Attribute bleiben
  sämtliche sechs generierten Dateien bytegleich zur archivierten Baseline.
  Wiederholte Generierung bytegleich, `--check` grün.
- Drei echte Packetnamen/IDs/Feldshapes nur im Archiv ergänzt, durch denselben
  tatsächlichen xtask erzeugt. Wiederholung bytegleich, `--check` grün.
  Ganze resultierende Proto-Crate mit striktem Alltarget-Clippy und Fmt sowie
  allen48 bestehenden Debugtests grün. Keine generierten Handedits.
- Alle35 gespeicherten read-only Produktionsdateien unverändert, einschließlich
  Spec, vollständiger Proto-/Codec-Crates und Cargo-/Lockfiles. Auch die
  archivierten betroffenen Manifest-/Lockfiles sind bytegleich.

Das Drei-Packet-Testschema beweist hier nur Emission/Compilation und die
enge Lintausnahme. Originale Wire-/Metadata-/Slot-/Sanitizeradmission,
Motionquantisierung, aktive Itementities und Gameplay bleiben NET-04.1 und
seine ausdrücklichen nachfolgenden Verbraucher. Es gibt keinen neuen
Performance-, Plugin- oder Drop-in-Nachweis.

Evidence unter `.codex-tmp/net04_0b/`: getrennte `baseline-generated/` und
`three-generated/`, `logs/`, `readonly-baseline.json`,
`baseline-generated-comparison.json`, `checks.json`, `three-packets.toml`
und abschließende Source-/Evidencefreezes. Produktions-Generated bleiben
absichtlich read-only bis Roots separater NET-04.0c-Adoption.
