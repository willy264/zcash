"""Build the pinned upstream Darkside server; install portable Go on Windows."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / ".tools"
CONFIG = json.loads((ROOT / "scripts/backend.json").read_text())
EXE = ".exe" if os.name == "nt" else ""
BACKEND = TOOLS / ("darksidewalletd" + EXE)
BUILD_STAMP = CONFIG["tag"] + ("-windows-truncate-v1" if os.name == "nt" else "")


def download(url: str, dest: Path) -> None:
    print(f"Downloading {url}", flush=True)
    with urllib.request.urlopen(url, timeout=90) as response, dest.open("wb") as output:
        shutil.copyfileobj(response, output)


def prepare() -> Path:
    TOOLS.mkdir(exist_ok=True)
    stamp = TOOLS / "backend-version.txt"
    if BACKEND.exists() and stamp.exists() and stamp.read_text().strip() == BUILD_STAMP:
        return BACKEND
    go = shutil.which("go")
    if go is None and os.name == "nt":
        go_path = TOOLS / "go/bin/go.exe"
        if not go_path.exists():
            with urllib.request.urlopen("https://go.dev/dl/?mode=json&include=all", timeout=45) as response:
                versions = json.load(response)
            release = next(v for v in versions if v["version"] == CONFIG["go_version"])
            package = next(f for f in release["files"] if f["os"] == "windows" and f["arch"] == "amd64" and f["kind"] == "archive")
            archive = TOOLS / package["filename"]
            download("https://go.dev/dl/" + package["filename"], archive)
            with archive.open("rb") as stream:
                checksum = hashlib.file_digest(stream, "sha256").hexdigest()
            if checksum != package["sha256"]:
                raise RuntimeError("Go archive checksum does not match go.dev")
            with zipfile.ZipFile(archive) as zipped:
                zipped.extractall(TOOLS)
        go = str(go_path)
    if go is None:
        raise RuntimeError(f"Install {CONFIG['go_version']} or newer, then rerun make demo")
    source = TOOLS / ("lightwalletd-" + CONFIG["tag"].removeprefix("v"))
    if not (source / "go.mod").exists():
        archive = TOOLS / "lightwalletd.tar.gz"
        download(f"https://codeload.github.com/zcash/lightwalletd/tar.gz/refs/tags/{CONFIG['tag']}", archive)
        with archive.open("rb") as stream:
            checksum = hashlib.file_digest(stream, "sha256").hexdigest()
        if checksum != CONFIG["source_sha256"]:
            raise RuntimeError("lightwalletd archive checksum does not match the pinned source")
        with tarfile.open(archive) as compressed:
            compressed.extractall(TOOLS, filter="data")
    if os.name == "nt":
        # Go's append-only Windows handles cannot truncate. Opening a separate
        # writable handle preserves upstream cache semantics without changing
        # transaction parsing, staging, or RPC behavior. Linux stays unmodified.
        cache_source = source / "common/cache.go"
        text = cache_source.read_text(encoding="utf-8")
        for member in ("lengths", "blocks"):
            old = f"c.{member}File.Truncate("
            replacement = f"os.Truncate(c.{member}Name, "
            if old not in text and replacement not in text:
                raise RuntimeError("Upstream Windows cache patch no longer applies; inspect the source")
            text = text.replace(old, replacement)
        cache_source.write_text(text, encoding="utf-8")
    env = os.environ.copy()
    env["CGO_ENABLED"] = "0"
    print(f"Building lightwalletd {CONFIG['tag']} with {go}", flush=True)
    subprocess.run([go, "build", "-o", str(BACKEND), "."], cwd=source, env=env, check=True)
    stamp.write_text(BUILD_STAMP + "\n")
    return BACKEND


if __name__ == "__main__":
    print(prepare())
