#!/usr/bin/env python3
"""Real Java plugin/client/installer contract against locally built images."""
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import time
import uuid
import zipfile

REPO = Path(__file__).resolve().parents[2]
DOCKER = shlex.split(os.environ.get("FERRUMC_DOCKER_COMMAND", "docker"))
IMAGE = "mchosting/ferrumc:v2-local"
NAME = "ferrumc-java-test-" + uuid.uuid4().hex[:12]
ARTIFACTS = Path(tempfile.mkdtemp(prefix="ferrumc-java-container-"))
DATA = ARTIFACTS / "data"
DATA.mkdir()


def docker(*args, check=True):
    result = subprocess.run(DOCKER + list(args), text=True, capture_output=True)
    if check and result.returncode:
        raise RuntimeError(result.stdout + result.stderr)
    return result


def hashes():
    return {str(p.relative_to(DATA)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in DATA.rglob("*") if p.is_file()}


def logs():
    output = docker("logs", NAME)
    return output.stdout + output.stderr


def run():
    egg = json.loads((REPO / "deploy/pterodactyl/egg-ferrumc-v2.json").read_text())
    installation = egg["scripts"]["installation"]
    script = ARTIFACTS / "install.sh"
    script.write_text(installation["script"])

    def install():
        docker("run", "--rm", "--mount", f"type=bind,src={DATA},dst=/mnt/server",
               "--mount", f"type=bind,src={script},dst=/mnt/install/install.sh,readonly",
               installation["container"], installation["entrypoint"], "/mnt/install/install.sh")

    install()
    assert (DATA / "java-plugins").is_dir()
    assert (DATA / "java-plugins").stat().st_uid == 1000
    fixture = Path(os.environ.get("FERRUMC_JAVA_FIXTURE_JAR", str(
        REPO / "server/java-bridge/fixture/target/standard-bukkit-fixture-0.1.0.jar")))
    assert fixture.is_file(), "Build the ordinary fixture with Maven verify first"
    (DATA / "java-plugins/fixture.jar").write_bytes(fixture.read_bytes())
    config = DATA / "config.toml"
    config.write_text('ops = ["JavaOperator"]\n'
                      'java_plugins_dir = "java-plugins"\n'
                      'java_bridge_jar = "/usr/local/share/ferrumc/ferrumc-java-bridge.jar"\n'
                      'java_executable = "java"\njava_bridge_timeout_ms = 10000\n'
                      'dashboard_enabled = false\n' + config.read_text())
    before = hashes()
    install()
    assert hashes() == before, "Reinstall must preserve configuration and JAR bytes"
    print("PASS: Wings installer creates Java directory and preserves config/plugin bytes", flush=True)

    docker("run", "--detach", "--name", NAME, "--stop-timeout", "60",
           "--publish", "127.0.0.1::25579/tcp", "--env", "SERVER_PORT=25579",
           "--mount", f"type=bind,src={DATA},dst=/home/container", IMAGE)
    deadline = time.monotonic() + 60
    while "listening for Minecraft clients" not in logs():
        state = json.loads(docker("inspect", NAME).stdout)[0]["State"]
        if not state["Running"] or time.monotonic() >= deadline:
            raise RuntimeError("Server failed readiness: " + logs())
        time.sleep(0.05)
    assert docker("exec", NAME, "id", "-u").stdout.strip() == "1000"
    assert docker("exec", NAME, "cat", "/proc/1/comm").stdout.strip() == "ferrumc"
    version = docker("exec", NAME, "java", "-version")
    assert 'version "21.' in version.stdout + version.stderr
    ports = json.loads(docker("inspect", NAME).stdout)[0]["NetworkSettings"]["Ports"]
    host_port = ports["25579/tcp"][0]["HostPort"]
    env = dict(os.environ, MC_JAVA_EXTERNAL="1", MC_HOST="127.0.0.1", MC_PORT=host_port)
    client = subprocess.run(["node", "java-plugin.mjs"], cwd=REPO / "tests/blackbox",
                            env=env, text=True, capture_output=True, timeout=90)
    (ARTIFACTS / "client.log").write_text(client.stdout + client.stderr)
    assert client.returncode == 0, "Independent Java client failed; inspect client.log"
    print("PASS: Java21 runs on Bookworm; UID1000, Rust PID1 and independent two-client gate", flush=True)

    packaged = ARTIFACTS / "packaged.jar"
    docker("cp", f"{NAME}:/usr/local/share/ferrumc/ferrumc-java-bridge.jar", str(packaged))
    with zipfile.ZipFile(packaged) as jar:
        assert "META-INF/third-party/paper/GPL-3.0.txt" in jar.namelist()
        assert "META-INF/third-party/FERRUMC-LICENSE.txt" in jar.namelist()
    docker("exec", NAME, "test", "-s", "/usr/local/share/ferrumc/source/paper-api-sources.jar")
    docker("exec", NAME, "test", "-s", "/usr/local/share/ferrumc/source/ferrumc-java-bridge-sources.jar")
    docker("exec", NAME, "test", "-s", "/usr/local/share/ferrumc/source/java-bridge/pom.xml")
    print("PASS: shaded API license notices and corresponding sources accompany the image", flush=True)

    docker("stop", "--time", "60", NAME)
    state = json.loads(docker("inspect", NAME).stdout)[0]["State"]
    assert state["ExitCode"] == 0
    server_logs = logs()
    (ARTIFACTS / "server.log").write_text(server_logs)
    assert "shutdown complete" in server_logs and server_logs.count("fixture stdout onDisable") == 1
    persisted = (DATA / "java-plugins/BridgeFixture/config.yml").read_text()
    for key, count in [("enabled", 1), ("disabled", 1), ("join-count", 2), ("quit-count", 2)]:
        assert f"{key}: {count}\n" in persisted, (key, persisted)
    before = hashes()
    install()
    assert hashes() == before, "Reinstall must preserve world and Java plugin state"
    print("PASS: SIGTERM disables once; quit hooks and config/world bytes survive reinstall", flush=True)


try:
    run()
finally:
    docker("rm", "--force", NAME, check=False)
    print("Artifacts:", ARTIFACTS, flush=True)
