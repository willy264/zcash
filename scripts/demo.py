"""Run real local services and preserve independent leaky/fixed evidence."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import time

from setup_backend import prepare

ROOT = Path(__file__).resolve().parents[1]
EXE = ".exe" if os.name == "nt" else ""


def run(args: list[str | Path], *, timeout: int = 1800, env: dict[str, str] | None = None, expected_returncode: int = 0) -> None:
    print("+ " + " ".join(map(str, args)), flush=True)
    result = subprocess.run(list(map(str, args)), cwd=ROOT, env=env, check=False, timeout=timeout)
    if result.returncode != expected_returncode:
        raise RuntimeError(f"Expected exit {expected_returncode}, got {result.returncode}: {args[0]}")
    if expected_returncode:
        print(f"Observed expected shielded-send refusal (exit {expected_returncode}).", flush=True)


def available(port: int) -> None:
    with socket.socket() as connection:
        try:
            connection.bind(("127.0.0.1", port))
        except OSError as error:
            raise RuntimeError(f"Local port {port} is already in use; stop that service before the demo") from error


def copy_walkthrough() -> None:
    report = json.loads((ROOT / "report/result.json").read_text(encoding="utf-8"))
    metadata_path = ROOT / "docs/demo.json"
    metadata = json.loads(metadata_path.read_text(encoding="utf-8")) if metadata_path.exists() else {}
    matches = set(metadata.get("leaks_exercised", [])) == set(report["exercised_checks"])
    for name in ("demo.mp4", "demo.vtt", "demo.json"):
        source = ROOT / "docs" / name
        if matches and source.exists():
            shutil.copyfile(source, ROOT / "report" / name)
        else:
            (ROOT / "report" / name).unlink(missing_ok=True)


class Service:
    def __init__(self, args: list[str | Path], log: Path, env: dict[str, str] | None = None):
        self.log = log
        self.output = log.open("wb")
        self.process = subprocess.Popen(list(map(str, args)), cwd=ROOT, env=env, stdout=self.output, stderr=subprocess.STDOUT)

    def check_alive(self) -> None:
        if self.process.poll() is not None:
            self.output.flush()
            raise RuntimeError(f"Service exited unexpectedly ({self.process.returncode}): {self.log}\n{self.log.read_text(errors='replace')[-5000:]}")

    def ready(self, port: int, seconds: int = 45) -> None:
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            self.check_alive()
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=0.3):
                    return
            except OSError:
                time.sleep(0.15)
        raise RuntimeError(f"Timed out waiting for port {port}; inspect {self.log}")

    def stop(self) -> None:
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        self.output.close()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs="?", choices=["demo", "fixture", "report"], default="demo")
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument("--release", action="store_true")
    options = parser.parse_args()
    if options.command in ("demo", "report"):
        # A failed rerun must never leave an old green report as the current result.
        for filename in ("index.html", "result.json"):
            (ROOT / "report" / filename).unlink(missing_ok=True)
    profile = "release" if options.release else "debug"
    binary = lambda name: ROOT / "target" / profile / (name + EXE)
    if not options.no_build:
        run(["cargo", "build", "--workspace", "--locked"] + (["--release"] if options.release else []))
    if options.command == "report":
        latest = json.loads((ROOT / ".runs/latest.json").read_text())
        run([binary("analyzer"), "--runs", ROOT / latest["run_dir"], "--ground-truth", "fixtures/ground_truth.json", "--output", "report"])
        copy_walkthrough()
        return
    run([binary("subject-wallet"), "--verify-fixture"], timeout=120)
    if options.command == "fixture":
        return
    backend_binary = prepare()
    for port in (9067, 9068, 19067, 18080):
        available(port)
    run_dir = ROOT / ".runs" / datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    run_dir.mkdir(parents=True)
    backend_data = run_dir / "backend-data"
    backend_data.mkdir()
    backend = Service([
        backend_binary, "--darkside-very-insecure", "--no-tls-very-insecure",
        "--grpc-bind-addr", "127.0.0.1:9067", "--http-bind-addr", "127.0.0.1:9068",
        "--data-dir", backend_data, "--log-file", run_dir / "darkside.log",
    ], run_dir / "backend-process.log")
    try:
        backend.ready(9067)
        run([binary("stager"), "--endpoint", "http://127.0.0.1:9067", "--fixtures", "fixtures"], timeout=120)
        backend.check_alive()
        for name, mode in (("leaky", "leaky"), ("fixed", "good")):
            directory = run_dir / name
            directory.mkdir()
            env = os.environ.copy()
            env.pop("DARKROOM_FIX", None)
            if name == "fixed":
                env["DARKROOM_FIX"] = "1"
            merchant = Service([binary("merchant-app"), "--listen", "127.0.0.1:18080", "--log", directory / "app.log"], directory / "merchant-process.log", env)
            proxy = None
            try:
                merchant.ready(18080)
                proxy = Service([
                    binary("observer-proxy"), "--listen", "127.0.0.1:19067",
                    "--upstream", "http://127.0.0.1:9067", "--log", directory / "observations.jsonl",
                ], directory / "proxy-process.log")
                proxy.ready(19067)
                run([
                    binary("subject-wallet"), "--mode", mode, "--endpoint", "http://127.0.0.1:19067",
                    "--output", directory / "wallet.json", "--memo", "--merchant-url", "http://127.0.0.1:18080/reconcile",
                    "--exercise-fallback",
                ], timeout=120, env=env, expected_returncode=3 if name == "fixed" else 0)
                proxy.check_alive()
                merchant.check_alive()
                backend.check_alive()
            finally:
                if proxy:
                    proxy.stop()
                merchant.stop()
            run([
                binary("analyzer"), "--expect", "leaky" if name == "leaky" else "clean",
                "--observations", directory / "observations.jsonl", "--app-log", directory / "app.log",
                "--wallet", directory / "wallet.json", "--ground-truth", "fixtures/ground_truth.json",
                "--output", directory / "analysis",
            ], timeout=120)
        run([binary("analyzer"), "--runs", run_dir, "--ground-truth", "fixtures/ground_truth.json", "--output", "report"], timeout=120)
        backend.check_alive()
        copy_walkthrough()
        (ROOT / ".runs/latest.json").write_text(json.dumps({"run_dir": run_dir.relative_to(ROOT).as_posix()}, indent=2) + "\n")
        print(f"Verified report: {ROOT / 'report/index.html'}\nCaptured evidence: {run_dir}", flush=True)
    finally:
        backend.stop()


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
        print(f"Darkroom failed: {error}", file=sys.stderr)
        sys.exit(1)
