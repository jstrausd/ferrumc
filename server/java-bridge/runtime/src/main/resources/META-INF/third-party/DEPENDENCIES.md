# Dependency provenance

The shaded executable embeds these API dependencies; it does not embed a Paper
server implementation. The Maven POM pins the exact Paper API artifact. Its
published POM pins transitive libraries. Run `mvn dependency:tree -pl runtime`
for the exact resolved graph.

- Paper API `1.21.8-R0.1-20250906.215025-55`: official Paper Maven repository,
  https://repo.papermc.io/repository/maven-public/ . Upstream licensing notice,
  GPLv3 and MIT contributor exception texts are bundled in `paper/`.
- Guava 33.3.1-jre, Gson 2.11.0, SnakeYAML 2.2, fastutil 8.5.15, Log4j API
  2.24.1, Apache Maven 3.9.6 / Resolver 1.9.18 and Apache HTTPClient 4.5.14:
  Apache License 2.0; available license/notice files preserved per artifact.
- Adventure 4.24.0, Examination 1.3.0, Option 1.1.0, JOML 1.10.8, SLF4J,
  BungeeCord chat, Brigadier, JetBrains annotations and related API/annotation
  libraries retain their respective upstream license files from published JARs
  in artifact-specific directories. Their copyrights are not replaced by the
  FerrumC MIT notice.

The Maven build preserves original dependency notices in distinct paths to
avoid silently losing files with the same `META-INF/LICENSE` name during shade.
Paper sources: https://repo.papermc.io/repository/maven-public/io/papermc/paper/paper-api/1.21.8-R0.1-SNAPSHOT/paper-api-1.21.8-R0.1-20250906.215025-55-sources.jar
Bridge sources: accompany the distribution as `ferrumc-java-bridge-0.1.0-sources.jar`.
