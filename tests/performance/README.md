# Real process baseline

`process-baseline.py` measures the same frozen **Release** image in native mode
and with its Java 21 compatibility host plus the ordinary Bukkit fixture. It
starts temporary local Docker containers with fresh persistent flat worlds;
each container progresses through 0, 4 and 8 standing independent Minecraft
1.21.8 clients at the same spawn. Two rounds reverse the order of the two modes.
The script does not build code or alter existing services, images or world data.

Install the existing pinned clients with pnpm 9.15.9 first:

```sh
pnpm --dir tests/blackbox install --frozen-lockfile
```

Ensure the tested image and the matching Maven fixture JAR are already built.
To reproduce the frozen 10 October baseline, use the preserved runtime below
or load its archived OCI image. The current `v2-local` tag refers to a newer
build. The fixture must match the saved artifact hash.

```sh
FERRUMC_DOCKER_COMMAND='sudo -n docker' \
FERRUMC_NAMESPACE_COMMAND='sudo -n nsenter' \
python3 tests/performance/process-baseline.py \
  --image sha256:e0cdc5f1e24240dec778a81903ff25fcbacb6adce95eadeb5ed7db24000a5b78 \
  --source-revision 7333f404f2957cfd27345a82daa3d814982f6c5d
```

The preserved local alias is `mchosting/ferrumc:baseline-7333`. Its runtime
manifest is the unchanged Linux amd64 runtime from the historical image index
`sha256:b11b4e3a7ee06c3348427c84e20e1ea2a9c08dc3c8ba9b84188ea27a43ec2b39`.
That original index also referenced an attestation that the local cache no
longer contains. The restored runtime therefore has the distinct image ID above;
its Rust binary and bridge JAR hashes match the frozen measurements exactly.
The valid local archive is `/tmp/ferrumc-baseline-7333-runtime.oci.tar`; keep a
copy before temporary files are removed. The historical raw data still records
the original index ID. This is preservation of the runtime, not a new measurement.

For a new build, pass its image and the revision that actually produced it.
Original measurements and their provenance remain unchanged when the harness
is fixed; rerunning the revised harness produces a separate result file.

Cleanup regression tests use Docker doubles and temporary local files:

```sh
python3 tests/performance/test_process_baseline.py
```

Defaults are view/simulation distance 4, 20 TPS, one-second sampling, 10-second
warmups and 30-second measurement windows (about eight minutes total). Coordinate
with other work so compilers, other benchmarks and application load do not run
concurrently. The default Docker and namespace commands omit `sudo`; configure
them according to local permissions. `nsenter` needs permission to enter an
owned container's network namespace. The production dashboard remains bound to
loopback inside that namespace and is never exposed via a host port.

The artifact directory printed at startup contains the frozen binary, host JAR
and fixture with SHA-256 hashes, complete JSON samples and summaries, exact
configuration, runtime/hardware/version metadata, client errors and server logs.
The image is resolved to its immutable ID before any server starts. Containers
stop gracefully and the script verifies both Rust/JVM process exit and closure
of every published Minecraft port. Cleanup discovers only containers carrying
the operation's unique ownership label, removes their immutable IDs, and checks
that none remain. Diagnostic log errors cannot skip removal attempts. A failure
preserves the diagnostic artifacts and records any cleanup failure.

RSS is sampled from the server process tree: Rust and JVM separately and their
sum. This sum can double-count shared pages and excludes filesystem cache,
container/runtime overhead and client memory. `VmSwap` is also recorded. Mean
CPU is the process tree's user+system CPU time divided by elapsed time, where
100% is one logical core; clients, Docker commands and the sampler are excluded.
Hardware/background load is recorded but the host is not made exclusive.

Startup readiness is end-to-end `docker run` through the readiness log, with
polling and Docker overhead. Filesystem/JRE caches are not cleared between runs.
Join timing runs from independent client creation
to the initial position, command graph and complete view. These are observable
local-client latencies, not isolated server callback timings. The tick endpoint
reports existing p50/p95/p99 over the last 600 driver ticks; that instrumented
section excludes network/chunk encoding, Java RPC, storage and snapshot work
outside it. Those percentiles are **not full server MSPT**. Effective TPS is
also independently derived from tick progress over the measurement window.

This small standing-player test is a starting point for regressions and for
measuring the baseline JVM cost. It exercises no Survival simulation, mob AI,
Redstone, normal terrain generation, active plugin workloads or broad Paper
compatibility. It proves neither a maximum player capacity nor a performance
advantage over Paper. Later comparisons need matching supported gameplay,
worlds, versions, workload, view distance and plugin behavior.

To check harness lifecycle only, use `--rounds 1 --warmup 1 --duration 2
--view-distance 1`. Short probes are not meaningful performance results.
