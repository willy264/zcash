#!/usr/bin/env python3
"""Record the verified local report with a timed, synthetic-voice walkthrough.

Windows authoring helper only. The report/demo pipeline does not require these
optional video dependencies. Install into .tools/video-tools:
  python -m pip install --target .tools/video-tools playwright imageio-ffmpeg
Then run after make demo:
  python scripts/record_demo.py
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import wave


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / ".tools" / "video-tools"))
DURATION = 120


def run(command: list[str]) -> None:
    subprocess.run(
        command,
        check=True,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )


def read_narration(path: Path) -> list[dict]:
    text = path.read_text(encoding="utf-8-sig")
    headings = list(
        re.finditer(r"^\*\*(\d+):(\d+)[–-](\d+):(\d+)[^\n]*\*\*\s*$", text, re.M)
    )
    segments = []
    for index, heading in enumerate(headings):
        start = 60 * int(heading[1]) + int(heading[2])
        end = 60 * int(heading[3]) + int(heading[4])
        stop = headings[index + 1].start() if index + 1 < len(headings) else len(text)
        body = " ".join(text[heading.end() : stop].strip().split())
        if not body or end <= start:
            raise ValueError("Narration has an empty or invalid timed segment")
        segments.append({"start": start, "end": end, "text": body})
    if not segments or segments[0]["start"] != 0 or segments[-1]["end"] != DURATION:
        raise ValueError("Narration must cover 0:00 through 2:00")
    if any(a["end"] != b["start"] for a, b in zip(segments, segments[1:])):
        raise ValueError("Narration segment intervals must be contiguous")
    return segments


def verify_report(report: Path) -> dict:
    if not report.is_file():
        raise FileNotFoundError("Run make demo first; no report exists to record")
    result = json.loads(report.with_name("result.json").read_text(encoding="utf-8"))
    if result.get("passed") is not True:
        raise ValueError("Refusing to record a report that did not pass its evidence gate")
    for name in ("leaky", "fixed"):
        run_result = result[name]
        if run_result.get("validated") is not True:
            raise ValueError(f"{name} evidence is not validated")
        findings = {finding["id"]: finding for finding in run_result["findings"]}
        if not all(findings[check]["exercised"] for check in ("A1", "A3")):
            raise ValueError("This narration requires real A1 and A3 evidence")
        leaked = {key for key, finding in findings.items() if finding["leaked"]}
        expected = {"A1", "A3"} if name == "leaky" else set()
        if leaked != expected or findings["A2"]["exercised"]:
            raise ValueError("Report findings differ from the two-leak narration")
    return result


def synthesize_audio(segments: list[dict], directory: Path, ffmpeg: str, voice: str) -> Path:
    manifest = directory / "segments.json"
    manifest.write_text(json.dumps(segments, ensure_ascii=False), encoding="utf-8")
    script = directory / "synthesize.ps1"
    script.write_text(
        r'''param([string]$SegmentsPath, [string]$AudioDirectory, [string]$VoiceName)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$taskNarrator = New-Object System.Speech.Synthesis.SpeechSynthesizer
$taskNarrator.SelectVoice($VoiceName)
$taskNarrator.Rate = 0
$taskSegments = Get-Content -LiteralPath $SegmentsPath -Raw -Encoding UTF8 | ConvertFrom-Json
$taskIndex = 0
foreach ($taskSegment in $taskSegments) {
    $taskWave = Join-Path $AudioDirectory ('voice-' + $taskIndex + '.wav')
    $taskNarrator.SetOutputToWaveFile($taskWave)
    $taskNarrator.Speak([string]$taskSegment.text)
    $taskNarrator.SetOutputToNull()
    $taskIndex += 1
}
$taskNarrator.Dispose()
''',
        encoding="utf-8-sig",
    )
    run([
        "powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script),
        "-SegmentsPath", str(manifest), "-AudioDirectory", str(directory), "-VoiceName", voice,
    ])
    command = [ffmpeg, "-hide_banner", "-loglevel", "warning", "-y"]
    command += ["-f", "lavfi", "-t", str(DURATION), "-i", "anullsrc=r=44100:cl=mono"]
    filters = []
    for index, segment in enumerate(segments):
        path = directory / f"voice-{index}.wav"
        with wave.open(str(path), "rb") as audio:
            seconds = audio.getnframes() / audio.getframerate()
        available = segment["end"] - segment["start"] - 0.5
        speed = max(1.0, seconds / available)
        if speed > 1.45:
            raise ValueError(f"Narration segment {index + 1} is too dense ({seconds:.1f}s in {available:.1f}s)")
        command += ["-i", str(path)]
        filters.append(
            f"[{index + 1}:a]aresample=44100,atempo={speed:.6f},"
            f"adelay={segment['start'] * 1000}:all=1[voice{index}]"
        )
    inputs = "[0:a]" + "".join(f"[voice{i}]" for i in range(len(segments)))
    filters.append(f"{inputs}amix=inputs={len(segments)+1}:normalize=0,loudnorm=I=-16:TP=-1.5:LRA=11[audio]")
    output = directory / "narration.wav"
    command += ["-filter_complex", ";".join(filters), "-map", "[audio]", "-t", str(DURATION), "-ar", "44100", "-ac", "1", "-c:a", "pcm_s16le", str(output)]
    run(command)
    return output


def record_page(report: Path, directory: Path, edge: Path) -> Path:
    from playwright.sync_api import sync_playwright

    # These are ordinary viewport changes on the genuine generated report.
    # No screenshot, finding, log, badge, or DOM content is fabricated.
    shots = [
        (0, "header.hero", 102),
        (13, "#a1", 24),
        (31, "#a1", 24),
        (48, "#a1 .comparison", 40),
        (61, "#a3", 24),
        (75, "#a3 .comparison", 40),
        (84, ".unexercised", 24),
        (94, ".methodology", 24),
        (109, ".run-command", 300),
        (116, "header.hero", 102),
    ]
    with sync_playwright() as p:
        browser = p.chromium.launch(executable_path=str(edge), headless=True)
        context = browser.new_context(
            viewport={"width": 1440, "height": 810},
            record_video_dir=str(directory),
            record_video_size={"width": 1440, "height": 810},
            device_scale_factor=1,
            reduced_motion="reduce",
        )
        page = context.new_page()
        page.goto(report.resolve().as_uri(), wait_until="load")
        page.locator("#a1 .state").first.wait_for()
        if not page.locator(".status.good").count():
            raise ValueError("Rendered report is not showing the expected passing gate")
        page.mouse.move(1430, 800)
        recording = page.video
        started = time.monotonic()
        for timestamp, selector, offset in shots:
            while (remaining := timestamp - (time.monotonic() - started)) > 0:
                page.wait_for_timeout(min(remaining, 1) * 1000)
            page.locator(selector).first.evaluate(
                "(el, offset) => window.scrollTo(0, el.getBoundingClientRect().top + window.scrollY - offset)", offset
            )
            print(f"Recorded cue {timestamp:03d}s: {selector}", flush=True)
        while (remaining := DURATION - (time.monotonic() - started)) > 0:
            page.wait_for_timeout(min(remaining, 1) * 1000)
        context.close()
        video = Path(recording.path())
        browser.close()
    return video


def subtitles(segments: list[dict]) -> str:
    def stamp(seconds: int) -> str:
        return f"00:{seconds // 60:02}:{seconds % 60:02}.000"
    return "WEBVTT\n\n" + "\n\n".join(
        f"{stamp(segment['start'])} --> {stamp(segment['end'])}\n{segment['text']}"
        for segment in segments
    ) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, default=ROOT / "report/index.html")
    parser.add_argument("--script", type=Path, default=ROOT / "docs/video-script.md")
    parser.add_argument("--output", type=Path, default=ROOT / "docs/demo.mp4")
    parser.add_argument("--voice", default="Microsoft Zira Desktop")
    parser.add_argument("--edge", type=Path, default=Path(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"))
    parser.add_argument("--check", action="store_true", help="Validate evidence/narration without recording")
    args = parser.parse_args()
    result = verify_report(args.report)
    segments = read_narration(args.script)
    if args.check:
        print(f"Ready: verified A1/A3 report and {len(segments)} narration segments covering {DURATION}s.")
        return
    if sys.platform != "win32" or not args.edge.is_file():
        raise RuntimeError("This optional authoring helper requires Windows System.Speech and Edge")
    import imageio_ffmpeg

    take = ROOT / ".tools" / "video-takes" / str(time.time_ns())
    take.mkdir(parents=True)
    ffmpeg = imageio_ffmpeg.get_ffmpeg_exe()
    print("Synthesizing the documented narration with a clearly synthetic system voice.", flush=True)
    narration = synthesize_audio(segments, take, ffmpeg, args.voice)
    print("Recording the verified report for 120 seconds.", flush=True)
    recording = record_page(args.report, take, args.edge)
    encoded = take / "demo.mp4"
    run([
        ffmpeg, "-hide_banner", "-loglevel", "warning", "-y", "-i", str(recording),
        "-i", str(narration), "-t", str(DURATION), "-map", "0:v:0", "-map", "1:a:0",
        "-vf", "scale=1280:720,format=yuv420p", "-r", "24", "-c:v", "libx264",
        "-preset", "medium", "-crf", "25", "-c:a", "aac", "-b:a", "96k",
        "-movflags", "+faststart", str(encoded),
    ])
    if encoded.stat().st_size > 15 * 1024 * 1024:
        raise ValueError("Video exceeded the 15 MiB artifact budget")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    os.replace(encoded, args.output)
    args.output.with_suffix(".vtt").write_text(subtitles(segments), encoding="utf-8")
    args.output.with_suffix(".json").write_text(json.dumps({
        "duration_seconds": DURATION,
        "narration": "Synthetic speech from the documented walkthrough script",
        "voice": args.voice,
        "report_sha256": hashlib.sha256(args.report.read_bytes()).hexdigest(),
        "result_sha256": hashlib.sha256(args.report.with_name("result.json").read_bytes()).hexdigest(),
        "source_commit": result.get("provenance", {}).get("commit"),
        "leaks_exercised": ["A1", "A3"],
        "transparent_fallback": "not exercised",
        "capture": "Real generated report in headless Microsoft Edge; viewport movement only",
    }, indent=2) + "\n", encoding="utf-8")
    print(f"Saved {args.output} ({args.output.stat().st_size / 1024 / 1024:.2f} MiB), plus captions and capture provenance.")


if __name__ == "__main__":
    main()
