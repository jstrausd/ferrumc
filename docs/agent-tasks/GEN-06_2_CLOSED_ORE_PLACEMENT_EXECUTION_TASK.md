# GEN-06.2 — freigegebene Umsetzung der geschlossenen Erzplatzierung

Root-Ausführungsauftrag, 12. Oktober 2026. `ore_primitives_runtime_r1` besitzt
exklusiv `server/crates/ferrumc-world/**`, sein neues Delivery-Dokument und
`.codex-tmp/gen06_2-runtime/**`. Keine anderen Crates, Abhängigkeiten,
Lockfileänderungen, Gitaktionen, weiteren Agents oder Appaktivierung.

Die Wartebedingung des [vollständigen ursprünglichen Runtimeauftrags](GEN-06_2_CLOSED_ORE_PLACEMENT_PRIMITIVES_RUNTIME_TASK.md)
ist erfüllt: Der [unabhängige Carver-Runtimekorrekturreview](GEN-05_11_CARVER_PRECHARGE_CORRECTION_REVIEW.md)
ist CLEAN_RUNTIME_REVIEW; Root hat die vollständigen Lieferungen geprüft und
alle364 Worlddateien als `8fc644bcaebb04109158e3204f46ec22a00854c0` integriert.
RR11-M1 ist geschlossen. Diese Freigabe ersetzt ausschließlich den Wartestatus;
alle Implementierungs-, Referenz-, Matrix-, Native- und Reviewpflichten des
ursprünglichen Auftrags bleiben unverändert.

Root liefert das echte readonly Archiv
`.codex-tmp/gen06_2-root-base/archive`:1561 Dateien,364 akzeptierte Worlddateien
und47 akzeptierte Registrydateien. `base.freeze.json` SHA256:
`a349a3cbebdc8bf39a7cb10d67078ce110e0870ada2f41376a8a3e5fd240e690`.
Private Builds verwenden dieses Archiv plus den eigenen vollständigen
World-Overlay, einen getrennten Target und Cargo locked/offline/jobs2.
Shared Registry-/Lockfile-WIP ist keine Buildbasis. Whole-Integrations-CI
[38294008459](https://github.com/jstrausd/ferrumc/actions/runs/38294008459)
läuft separat; ihr Erfolg wird vor Abschluss nicht behauptet.

Der Auftrag umfasst den vollständigen57-Ressourcen-Katalog, Original-RNG und
Lazy-Platzierungsprimitive mit tatsächlichem gemeinsamen Meter, sealed Lease,
Pending-Completion und Refusal-/Cleanup-/Unwindverträgen. Kein produktiver
Biome- oder Orewriteconsumer. Single-key1Mi und die messpflichtigen Native-
Designlimits16KiB Katalog/16KiB Gesamtscratch/32KiB neuer Peak/49152B mit altem
Katalog/512B Cursor bleiben unverändert. Konkrete unvereinbare Layouts sind Root
vor einem Freeze mit der kleinsten belegten Korrektur zu melden.

[GEN-06.3 Revision3](GEN-06_3_ORE_ALGORITHM_REVISION3_REVIEW.md) ist nur auf
kontrollierter Vorbereitungs-/Originaloracle-Ebene angenommen. Tatsächlicher
Getter-/Parent-/WGhistoryproducer, Regioninterfaces/-limits, H2 und wholeFEATURES
bleiben getrennte Gates. Der vollständige ursprüngliche GEN-06.1/Revision2-
Vertrag und seine Reviews müssen gelesen und umgesetzt werden.

Ein ganzer unveränderlicher Worldkandidat ist vor den vollständigen Finalchecks
anzukündigen. Originalfixtures, historische Freezes und Fehlversuche bleiben
erhalten. Alle erforderlichen Original-/Budget-/Lifecycle-/Nativefälle werden
auf der echten privaten Produktionslogik geprüft; Source-/Evidencefreeze und
anschließende unabhängige Runtimeprüfung sind getrennte Schritte. Root allein
integriert. Natürliche Weltgenerierung, Survival und Live-Performance bleiben offen.
