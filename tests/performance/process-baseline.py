#!/usr/bin/env python3
"""Measure frozen Release FerrumC processes with 0/4/8 standing clients.

Requires Linux /proc, Docker, nsenter, curl, Python 3 and the pinned pnpm install.
Creates only owned temporary containers/data. No production service is touched.
"""
import argparse
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import select
import shlex
import shutil
import signal
import socket
import subprocess
import tempfile
import time
import uuid

REPO = Path(__file__).resolve().parents[2]
CLOCK_TICKS = os.sysconf("SC_CLK_TCK")
PAGE_BYTES = os.sysconf("SC_PAGE_SIZE")
MIB = 1024 * 1024
OWNERSHIP_LABEL = "ferrumc.performance.owner"


def utc():
    return dt.datetime.now(dt.timezone.utc).isoformat()


def run(command, *, timeout=30, check=True):
    result = subprocess.run(command, text=True, capture_output=True, timeout=timeout)
    if check and result.returncode:
        raise RuntimeError(f"Command failed: {shlex.join(command)}\n{result.stdout}{result.stderr}")
    return result


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def percentile(values, fraction):
    """Nearest-rank percentile; deliberately no inferred distribution."""
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def process_tree(root):
    """Read processes, not threads. Follow children of every parent task because
    the JVM can be spawned by a Tokio worker rather than the main thread.
    """
    processes = []
    pending = [root]
    seen = set()
    while pending:
        pid = pending.pop()
        if pid in seen:
            continue
        seen.add(pid)
        proc = Path(f"/proc/{pid}")
        try:
            stat = (proc / "stat").read_text()
            # comm can contain spaces and ')' characters; the final ')' ends it.
            fields = stat[stat.rindex(")") + 2:].split()
            comm = (proc / "comm").read_text().strip()
            status = dict(line.split(":", 1) for line in (proc / "status").read_text().splitlines())
            processes.append({"pid": pid, "start_ticks": int(fields[19]), "comm": comm,
                              "cpu_ticks": int(fields[11]) + int(fields[12]),
                              "rss_bytes": int(fields[21]) * PAGE_BYTES,
                              "swap_bytes": int(status.get("VmSwap", "0 kB").split()[0]) * 1024,
                              "threads": int(status["Threads"])})
            for task in (proc / "task").iterdir():
                try:
                    pending.extend(int(child) for child in (task / "children").read_text().split())
                except (FileNotFoundError, ProcessLookupError):
                    pass
        except (FileNotFoundError, ProcessLookupError):
            continue
    if not processes or processes[0]["comm"] != "ferrumc":
        raise RuntimeError(f"Rust server process {root} disappeared")
    if any(proc["comm"] not in {"ferrumc", "java"} for proc in processes):
        raise RuntimeError(f"Unexpected server descendant: {processes}")
    return processes


def host_counters():
    fields = [int(value) for value in Path("/proc/stat").read_text().splitlines()[0].split()[1:9]]
    return {"total": sum(fields), "idle": fields[3] + fields[4],
            "loadavg": Path("/proc/loadavg").read_text().strip()}


def cpu_delta(previous, current):
    before = {(proc["pid"], proc["start_ticks"]): proc for proc in previous}
    totals = {"rust": 0.0, "java": 0.0}
    for proc in current:
        prior = before.get((proc["pid"], proc["start_ticks"]))
        if prior is None:
            raise RuntimeError("Server process tree changed during a steady measurement window")
        lane = "rust" if proc["comm"] == "ferrumc" else "java"
        totals[lane] += (proc["cpu_ticks"] - prior["cpu_ticks"]) / CLOCK_TICKS
    if len(before) != len(current):
        raise RuntimeError("Server process tree lost a process during measurement")
    return totals


class Clients:
    def __init__(self, port, view, artifacts):
        self.log = (artifacts / "clients.stderr.log").open("w")
        self.process = subprocess.Popen(["node", str(REPO / "tests/performance/clients.mjs"),
                                         "127.0.0.1", str(port), str(view)],
                                        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=self.log, text=True, bufsize=1)

    def request(self, value):
        self.process.stdin.write(json.dumps(value) + "\n")
        self.process.stdin.flush()
        readable, _, _ = select.select([self.process.stdout], [], [], 45)
        if not readable:
            raise RuntimeError("Client harness timed out; inspect clients.stderr.log")
        line = self.process.stdout.readline()
        if not line:
            raise RuntimeError("Client harness exited; inspect clients.stderr.log")
        return json.loads(line)

    def close(self):
        try:
            if self.process.poll() is None:
                self.request({"op": "stop"})
            self.process.wait(timeout=10)
            if self.process.returncode:
                raise RuntimeError(f"Client harness exited {self.process.returncode}")
        finally:
            if self.process.poll() is None:
                self.process.kill()
                self.process.wait(timeout=10)
            self.process.stdin.close()
            self.process.stdout.close()
            self.log.close()


class Baseline:
    def __init__(self, args):
        self.args = args
        self.docker_command = shlex.split(args.docker_command)
        self.namespace_command = shlex.split(args.namespace_command)
        self.artifacts = Path(tempfile.mkdtemp(prefix="ferrumc-process-baseline-"))
        inspected = json.loads(self.docker("image", "inspect", args.image).stdout)[0]
        self.image = inspected["Id"]  # Never use a mutable tag after this point.
        frozen = self.artifacts / "frozen"
        frozen.mkdir()
        fixture = args.fixture.resolve(strict=True)
        shutil.copyfile(fixture, frozen / "fixture.jar")
        extract_owner = uuid.uuid4().hex
        extract = "ferrumc-baseline-extract-" + extract_owner[:12]
        try:
            created = self.docker("create", "--name", extract,
                                  "--label", f"{OWNERSHIP_LABEL}={extract_owner}", self.image)
            extract_id = created.stdout.strip()
            for source, name in [("/usr/local/bin/ferrumc", "ferrumc"),
                                 ("/usr/local/share/ferrumc/ferrumc-java-bridge.jar", "bridge.jar")]:
                self.docker("cp", f"{extract_id}:{source}", str(frozen / name))
        finally:
            self.remove_owned_containers(extract_owner)
        cpu = next((line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines()
                    if line.startswith("model name")), "unknown")
        self.result = {
            "schema": 1, "started_utc": utc(), "source_revision": args.source_revision,
            "image_requested": args.image, "image_id": self.image, "image_created": inspected["Created"],
            "frozen_sha256": {path.name: sha256(path) for path in frozen.iterdir()},
            "hardware": {"cpu_model": cpu, "logical_cpus": os.cpu_count(), "kernel": platform.release(),
                         "meminfo_start": Path("/proc/meminfo").read_text(),
                         "cpu_scaling_governors": {str(path): path.read_text().strip() for path in
                             Path("/sys/devices/system/cpu").glob("cpu[0-9]*/cpufreq/scaling_governor")}},
            "tools": {"python": platform.python_version(), "node": run(["node", "--version"]).stdout.strip(),
                      "docker": self.docker("--version").stdout.strip(),
                      "curl": run(["curl", "--version"]).stdout.splitlines()[0],
                      "nsenter": run(self.namespace_command + ["--version"]).stdout.strip()},
            "settings": {"rounds": args.rounds, "warmup_seconds": args.warmup,
                         "measurement_seconds": args.duration, "sample_interval_seconds": 1,
                         "view_distance": args.view_distance, "clients": [0, 4, 8], "tps": 20,
                         "spawn": [8, 64, 8], "compression": "disabled", "java_flags": "image defaults"},
            "runs": [], "artifacts": str(self.artifacts),
        }
        print(f"Frozen image: {self.image}\nArtifacts: {self.artifacts}", flush=True)
        self.save()

    def docker(self, *args, timeout=30, check=True):
        return run(self.docker_command + list(args), timeout=timeout, check=check)

    def remove_owned_containers(self, owner):
        """Remove only this operation's labeled container IDs, then verify exit.

        A timed-out create/run can leave a container without returning its ID.
        The unique label recovers exactly our containers without trusting a name
        that might already exist. Attempt every removal even if one fails.
        """
        def owned_ids():
            return self.docker("container", "ls", "--all", "--no-trunc",
                               "--filter", f"label={OWNERSHIP_LABEL}={owner}",
                               "--format", "{{.ID}}").stdout.split()

        failures = []
        for container_id in owned_ids():
            try:
                self.docker("rm", "--force", container_id, timeout=20)
            except Exception as error:
                failures.append(str(error))
        remaining = owned_ids()
        if failures or remaining:
            raise RuntimeError(f"Owned container removal failed: {failures}; remaining IDs: {remaining}")

    def save(self):
        (self.artifacts / "results.json").write_text(json.dumps(self.result, indent=2) + "\n")

    def snapshot(self, root):
        # The production dashboard deliberately rejects non-loopback binds.
        # Enter only this owned process's network namespace; neither expose the
        # dashboard port nor weaken its bind guard just to collect a baseline.
        response = run(self.namespace_command + ["--target", str(root), "--net", "curl",
                       "--silent", "--show-error", "--fail", "--noproxy", "*", "--max-time", "3",
                       "http://127.0.0.1:9090/api/snapshot"], timeout=5)
        return json.loads(response.stdout)

    def sample(self, root):
        processes = process_tree(root)
        now = time.monotonic()
        snapshot = self.snapshot(root)
        selected = ["tick", "tps", "players_online", "tick_p50_ms", "tick_p95_ms", "tick_p99_ms",
                    "chunks_loaded", "chunk_sent_total", "chunk_unloaded_total",
                    "network_outbound_queue_len_max", "storage_flush_ms_last", "storage_flush_ms_avg",
                    "decode_errors_recent", "decode_errors_overflow", "block_breaks", "block_places"]
        row = {"monotonic": now, "utc": utc(), "processes": processes, "host": host_counters(),
               "snapshot": {key: snapshot[key] for key in selected}}
        for lane, comm in [("rust", "ferrumc"), ("java", "java")]:
            for metric in ["rss_bytes", "swap_bytes", "threads"]:
                row[f"{lane}_{metric}"] = sum(proc[metric] for proc in processes if proc["comm"] == comm)
        return row

    def measure(self, root, count):
        samples = [self.sample(root)]
        if samples[0]["snapshot"]["players_online"] != count:
            raise RuntimeError(f"Expected {count} admitted players: {samples[0]['snapshot']}")
        start = samples[0]["monotonic"]
        for index in range(1, self.args.duration + 1):
            time.sleep(max(0, start + index - time.monotonic()))
            current = self.sample(root)
            previous = samples[-1]
            elapsed = current["monotonic"] - previous["monotonic"]
            cpu = cpu_delta(previous["processes"], current["processes"])
            current["interval_seconds"] = elapsed
            for lane in ["rust", "java"]:
                current[f"{lane}_cpu_seconds"] = cpu[lane]
                current[f"{lane}_cpu_percent"] = 100 * cpu[lane] / elapsed
            host_delta = current["host"]["total"] - previous["host"]["total"]
            host_idle_delta = current["host"]["idle"] - previous["host"]["idle"]
            current["host_busy_percent"] = 100 * (host_delta - host_idle_delta) / host_delta
            snapshot = current["snapshot"]
            if snapshot["players_online"] != count or snapshot["tick"] <= previous["snapshot"]["tick"]:
                raise RuntimeError("Player count changed or server stopped ticking during measurement")
            if snapshot["decode_errors_recent"] or snapshot["decode_errors_overflow"]:
                raise RuntimeError("Protocol decoding errors invalidate this workload")
            samples.append(current)
        elapsed = samples[-1]["monotonic"] - start
        summary = {"sample_count": len(samples), "measured_seconds": elapsed,
                   "host_busy_percent_mean": sum(row["host_busy_percent"] for row in samples[1:]) / (len(samples) - 1),
                   "tick_endpoint": samples[-1]["snapshot"],
                   "observed_tps_mean": (samples[-1]["snapshot"]["tick"] - samples[0]["snapshot"]["tick"]) / elapsed}
        for lane in ["rust", "java", "total"]:
            rss = [row["rust_rss_bytes"] + row["java_rss_bytes"] if lane == "total" else row[f"{lane}_rss_bytes"]
                   for row in samples]
            summary[f"{lane}_rss_mib_p50"] = percentile(rss, 0.5) / MIB
            summary[f"{lane}_rss_mib_p95"] = percentile(rss, 0.95) / MIB
            summary[f"{lane}_rss_mib_max"] = max(rss) / MIB
            summary[f"{lane}_swap_mib_max"] = max(
                row["rust_swap_bytes"] + row["java_swap_bytes"] if lane == "total" else row[f"{lane}_swap_bytes"]
                for row in samples) / MIB
            summary[f"{lane}_cpu_percent_mean"] = 100 * sum(
                row["rust_cpu_seconds"] + row["java_cpu_seconds"] if lane == "total" else row[f"{lane}_cpu_seconds"]
                for row in samples[1:]) / elapsed
        return {"clients": count, "summary": summary, "samples": samples}

    def one_run(self, mode, round_number):
        owner = uuid.uuid4().hex
        name = "ferrumc-baseline-" + owner[:12]
        artifacts = self.artifacts / f"round-{round_number}-{mode}"
        data = artifacts / "data"
        plugins = data / "java-plugins"
        plugins.mkdir(parents=True)
        config = ["dashboard_enabled = true", 'dashboard_bind = "127.0.0.1:9090"',
                  f"view_distance = {self.args.view_distance}", f"simulation_distance = {self.args.view_distance}",
                  "spawn_chunk_radius = 1", "ticks_per_second = 20", "max_connections = 16",
                  "chunk_stream_interval_ms = 50", "builtin_plugins = true", 'world_dir = "world"']
        if mode == "java":
            shutil.copyfile(self.artifacts / "frozen/fixture.jar", plugins / "fixture.jar")
            config += ['java_plugins_dir = "java-plugins"',
                       'java_bridge_jar = "/usr/local/share/ferrumc/ferrumc-java-bridge.jar"',
                       'java_executable = "java"', "java_bridge_timeout_ms = 10000"]
        config += ["", "[access]", "per_ip_connection_limit = 0", ""]
        (data / "config.toml").write_text("\n".join(config))
        record = {"mode": mode, "round": round_number, "started_utc": utc(),
                  "config": "\n".join(config), "phases": []}
        self.result["runs"].append(record)
        clients = None
        root = None
        port = None
        container_id = None
        try:
            started = time.monotonic()
            created = self.docker("run", "--detach", "--name", name, "--stop-timeout", "60",
                        "--label", f"{OWNERSHIP_LABEL}={owner}",
                        "--publish", "127.0.0.1::25565/tcp",
                        "--mount", f"type=bind,src={data},dst=/home/container", self.image)
            container_id = created.stdout.strip()
            inspect = json.loads(self.docker("inspect", container_id).stdout)[0]
            ports = inspect["NetworkSettings"]["Ports"]
            port = int(ports["25565/tcp"][0]["HostPort"])
            root = inspect["State"]["Pid"]
            deadline = started + 60
            while True:
                log = self.docker("logs", container_id)
                if "listening for Minecraft clients" in log.stdout + log.stderr:
                    record["startup_ready_ms"] = (time.monotonic() - started) * 1000
                    break
                if time.monotonic() >= deadline:
                    raise RuntimeError(f"Readiness timeout\n{log.stdout}{log.stderr}")
                if not json.loads(self.docker("inspect", container_id).stdout)[0]["State"]["Running"]:
                    raise RuntimeError(f"Server exited before readiness\n{log.stdout}{log.stderr}")
                time.sleep(0.05)
            # Record the actual image runtime, not a potentially different host JDK.
            java = self.docker("exec", container_id, "java", "-version")
            record["java_version"] = (java.stdout + java.stderr).strip()
            owned = process_tree(root)
            record["owned_processes"] = [{"pid": proc["pid"], "start_ticks": proc["start_ticks"],
                                           "comm": proc["comm"]} for proc in owned]
            java_count = sum(proc["comm"] == "java" for proc in owned)
            if java_count != (1 if mode == "java" else 0):
                raise RuntimeError(f"Expected Rust plus {1 if mode == 'java' else 0} JVMs: {owned}")
            clients = Clients(port, self.args.view_distance, artifacts)
            for count in [0, 4, 8]:
                response = clients.request({"op": "clients", "count": count})
                if response["clients"] != count:
                    raise RuntimeError("Client control response did not match the requested count")
                print(f"Round {round_number} {mode}: {count} clients; warmup {self.args.warmup}s", flush=True)
                time.sleep(self.args.warmup)
                phase = self.measure(root, count)
                phase["joins"] = response["joins"]
                record["phases"].append(phase)
                summary = phase["summary"]
                print(f"  RSS Rust/Java/total p50 MiB: {summary['rust_rss_mib_p50']:.2f}/"
                      f"{summary['java_rss_mib_p50']:.2f}/{summary['total_rss_mib_p50']:.2f}; "
                      f"CPU total {summary['total_cpu_percent_mean']:.2f}% of one core; "
                      f"TPS {summary['observed_tps_mean']:.3f}", flush=True)
                self.save()
            clients.close()
            clients = None
            self.docker("stop", "--time", "60", container_id, timeout=70)
            state = json.loads(self.docker("inspect", container_id).stdout)[0]["State"]
            if state["ExitCode"] != 0:
                raise RuntimeError(f"Server stopped with nonzero exit: {state}")
            record["server_exit_code"] = state["ExitCode"]
            logs = self.docker("logs", container_id)
            combined = logs.stdout + logs.stderr
            (artifacts / "server.log").write_text(combined)
            if "shutdown complete" not in combined:
                raise RuntimeError("Missing graceful shutdown marker")
            if mode == "java" and combined.count("fixture stdout onDisable") != 1:
                raise RuntimeError("Java fixture was not disabled exactly once")
            record["finished_utc"] = utc()
        except BaseException as error:
            record["error"] = str(error)
            raise
        finally:
            if clients is not None:
                try:
                    clients.close()
                except Exception as error:
                    record["client_cleanup_error"] = str(error)
            # Diagnostic collection must not prevent owned-container removal.
            if container_id is not None:
                try:
                    logs = self.docker("logs", container_id, check=False)
                    if logs.returncode:
                        raise RuntimeError(f"Owned container log collection failed: {logs.stderr}")
                    (artifacts / "server.log").write_text(logs.stdout + logs.stderr)
                except Exception as error:
                    record["log_collection_error"] = str(error)
            record["cleanup_verified"] = False
            try:
                self.remove_owned_containers(owner)
                record["cleanup_verified"] = True
            except Exception as error:
                record["container_cleanup_error"] = str(error)
            if root is not None and Path(f"/proc/{root}").exists():
                record["cleanup_verified"] = False
            for proc in record.get("owned_processes", []):
                try:
                    stat = Path(f"/proc/{proc['pid']}/stat").read_text()
                    fields = stat[stat.rindex(")") + 2:].split()
                    if int(fields[19]) == proc["start_ticks"]:
                        record["cleanup_verified"] = False
                except (FileNotFoundError, ProcessLookupError):
                    pass
            for published in [port]:
                if published is not None:
                    with socket.socket() as sock:
                        sock.settimeout(1)
                        if sock.connect_ex(("127.0.0.1", published)) == 0:
                            record["cleanup_verified"] = False
            self.save()
            if not record["cleanup_verified"]:
                raise RuntimeError("Owned process or published port remains after cleanup")

    def execute(self):
        for round_number in range(1, self.args.rounds + 1):
            # Reverse the order on even rounds to reduce a simple cache/order bias.
            for mode in (["native", "java"] if round_number % 2 else ["java", "native"]):
                self.one_run(mode, round_number)
        self.result["finished_utc"] = utc()
        self.result["hardware"]["meminfo_end"] = Path("/proc/meminfo").read_text()
        self.save()
        print(f"PASS: all owned processes stopped and ports closed. Results: {self.artifacts / 'results.json'}", flush=True)


def main():
    def terminate(signum, _frame):
        # Convert normal termination into stack unwinding so owned clients and
        # containers follow the same finally cleanup as a failed measurement.
        raise KeyboardInterrupt(f"Interrupted by signal {signum}")

    signal.signal(signal.SIGTERM, terminate)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", default="mchosting/ferrumc:v2-local")
    parser.add_argument("--source-revision", required=True,
                        help="Recorded revision that produced the image; verify against build/validation records")
    parser.add_argument("--fixture", type=Path,
                        default=REPO / "server/java-bridge/fixture/target/standard-bukkit-fixture-0.1.0.jar")
    parser.add_argument("--docker-command", default=os.environ.get("FERRUMC_DOCKER_COMMAND", "docker"))
    parser.add_argument("--namespace-command", default=os.environ.get("FERRUMC_NAMESPACE_COMMAND", "nsenter"))
    parser.add_argument("--rounds", type=int, default=2)
    parser.add_argument("--warmup", type=int, default=10)
    parser.add_argument("--duration", type=int, default=30)
    parser.add_argument("--view-distance", type=int, default=4)
    args = parser.parse_args()
    if not (1 <= args.rounds <= 4 and 1 <= args.warmup <= 60 and 1 <= args.duration <= 120
            and 0 <= args.view_distance <= 8):
        parser.error("rounds 1..4; warmup 1..60; duration 1..120; view-distance 0..8")
    Baseline(args).execute()


if __name__ == "__main__":
    main()
