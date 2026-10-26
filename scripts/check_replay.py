#!/usr/bin/env python3
"""Verify the generated replay in a real browser, without a product build toolchain.

Optional review dependency: playwright (also found in .tools/video-tools).
Uses installed Microsoft Edge on Windows, or Playwright Chromium elsewhere.
Run against file:// and an ephemeral local HTTP server:
  python scripts/check_replay.py --report report/index.html
Run the same checks against a deployed preview:
  python scripts/check_replay.py --url https://example.test/preview/
"""
from __future__ import annotations

import argparse
import base64
from contextlib import contextmanager
import hashlib
from html.parser import HTMLParser
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import re
import sys
import threading
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / ".tools" / "video-tools"))
FILENAMES = {
    f"{profile}-{name}"
    for profile in ("leaky", "fixed")
    for name in ("observations.jsonl", "app.log", "wallet.json")
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def fetch(url: str) -> bytes:
    request = urllib.request.Request(url, headers={"Cache-Control": "no-cache"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read()


class EmbeddedHTML(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=False)
        self.policy = ""
        self.scripts: list[dict] = []
        self.current: dict | None = None

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attributes = dict(attrs)
        if tag == "meta" and attributes.get("http-equiv", "").lower() == "content-security-policy":
            self.policy = attributes.get("content", "")
        if tag == "script":
            self.current = {"attrs": attributes, "text": ""}

    def handle_data(self, data: str) -> None:
        if self.current is not None:
            self.current["text"] += data

    def handle_endtag(self, tag: str) -> None:
        if tag == "script" and self.current is not None:
            self.scripts.append(self.current)
            self.current = None


def load_evidence(url: str, expected_commit: str | None) -> tuple[dict, dict[str, bytes], dict]:
    raw_html = fetch(url)
    parsed = EmbeddedHTML()
    parsed.feed(raw_html.decode("utf-8"))
    embedded = [item for item in parsed.scripts if item["attrs"].get("id") == "replay-data"]
    require(len(embedded) == 1, "Expected exactly one inert replay-data element")
    require(embedded[0]["attrs"].get("type") == "application/json", "Evidence must be inert JSON")
    data = json.loads(embedded[0]["text"])
    require(data["schema_version"] == 1, "Unexpected embedded evidence schema")
    require(set(data["files"]) == FILENAMES, "Expected all six original evidence files")
    files = {}
    for name in sorted(FILENAMES):
        record = data["files"][name]
        original = base64.b64decode(record["base64"], validate=True)
        require(len(original) == record["byte_length"], f"Wrong byte length: {name}")
        require(hashlib.sha256(original).hexdigest() == record["sha256"], f"Wrong SHA-256: {name}")
        require(original == fetch(urllib.parse.urljoin(url, name)), f"Embedded bytes differ from download: {name}")
        original.decode("utf-8", errors="strict")
        files[name] = original
    report = data["report"]
    require(report == json.loads(fetch(urllib.parse.urljoin(url, "result.json"))), "Embedded result differs from result.json")
    require(report["passed"] is True, "Evidence gate did not pass")
    require(set(report["exercised_checks"]) == {"A1", "A2", "A3"}, "Three checks must be exercised")
    if expected_commit:
        require(report["provenance"]["commit"] == expected_commit, "Unexpected producing commit")
    directives = {part.strip().split()[0]: part.strip().split()[1:]
                  for part in parsed.policy.split(";") if part.strip()}
    script_policy = directives.get("script-src", [])
    require("'unsafe-inline'" not in script_policy and "'unsafe-eval'" not in script_policy,
            "Replay must not permit arbitrary inline scripts or eval")
    require(directives.get("connect-src", directives.get("default-src")) == ["'none'"],
            "Replay must not allow network connections")
    executable = [item for item in parsed.scripts if item["attrs"].get("type", "") != "application/json"]
    require(len(executable) == 1, "Expected one trusted inline replay script")
    require("src" not in executable[0]["attrs"], "Replay script must be embedded")
    # HTML parsing normalizes CRLF before CSP hashes the script's text.
    script = executable[0]["text"].replace("\r\n", "\n").replace("\r", "\n")
    digest = base64.b64encode(hashlib.sha256(script.encode("utf-8")).digest()).decode("ascii")
    require(f"'sha256-{digest}'" in script_policy, "CSP hash does not match the executed script")
    summary = {"report_sha256": hashlib.sha256(raw_html).hexdigest(),
               "commit": report["provenance"]["commit"],
               "ci_run_url": report["provenance"]["ci_run_url"],
               "files": {name: hashlib.sha256(raw).hexdigest() for name, raw in files.items()},
               "inline_script_sha256": digest}
    return report, files, summary


@contextmanager
def local_server(directory: Path):
    class Handler(SimpleHTTPRequestHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, directory=str(directory), **kwargs)

        def log_message(self, *_args) -> None:
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}/index.html"
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


def source_lines(raw: bytes) -> list[str]:
    lines = raw.decode("utf-8").split("\n")
    if lines[-1] == "":
        lines.pop()
    return [line[:-1] if line.endswith("\r") else line for line in lines]


def check_stream(page, stream, lines: list[str], source: str, inference: str) -> None:
    rows = stream.locator("details.replay-row")
    require(rows.count() == len(lines), f"Replay omitted {source} records")
    for index, original in enumerate(lines):
        row = rows.nth(index)
        record = json.loads(original)
        expected_seq = record["seq"] if source == "rpc" else index + 1
        require(row.get_attribute("data-seq") == str(expected_seq), "Replay sequence was invented or reordered")
        require(row.locator("pre.raw-record").text_content() == original, "Drawer changed the original JSONL record")
        if row.get_attribute("data-highlight") == "true":
            require(row.get_attribute("open") is not None, "Finding evidence must be expanded initially")
            require(row.locator(".record-inference").inner_text() == inference, "Finding explanation changed")
    stream.locator('[data-action="restart"]').click()
    require(stream.locator("details.replay-row:visible").count() == 0, "Restart must hide all records")
    status = page.locator("#replay-status")
    require("0 of" in status.inner_text(), "Restart was not announced")
    next_button = stream.locator('[data-action="next"]')
    verdict = page.locator("#selected-findings").inner_text()
    for index in range(len(lines)):
        next_button.focus()
        page.keyboard.press("Enter")
        require(stream.locator("details.replay-row:visible").count() == index + 1,
                "Keyboard Next must reveal exactly one captured record")
        require(f"{index + 1} of" in status.inner_text(), "Keyboard stepping was not announced")
        row = rows.nth(index)
        require(row.get_attribute("open") is not None and row.locator("pre.raw-record").is_visible(),
                "Next must reveal the actual raw record")
        if row.get_attribute("data-highlight") == "true":
            require(inference in status.inner_text(), "Finding inference was not announced")
        require(page.locator("#selected-findings").inner_text() == verdict, "Stepping changed the complete-run verdict")
    stream.locator('[data-action="restart"]').click()
    stream.locator('[data-action="all"]').click()
    require(stream.locator("details.replay-row:visible").count() == len(lines), "Show all must restore every record")


def check_browser(browser, url: str, args, screenshots: bool) -> dict:
    from playwright.sync_api import expect

    report, files, result = load_evidence(url, args.expected_commit)
    result["url"] = url
    result["checks"] = []
    errors, violations, network = [], [], []
    context = browser.new_context(viewport={"width": 1440, "height": 1000},
                                  device_scale_factor=1, reduced_motion="reduce")
    context.grant_permissions(["clipboard-read", "clipboard-write"])
    page = context.new_page()
    page.add_init_script("window.__reviewCsp = []; addEventListener('securitypolicyviolation', e => window.__reviewCsp.push(e.violatedDirective + ': ' + e.blockedURI));")
    page.on("pageerror", lambda error: errors.append(str(error)))
    page.on("console", lambda message: errors.append(message.text) if message.type == "error" else None)
    document_loaded = False

    def route_request(route) -> None:
        request = route.request
        if not document_loaded and request.is_navigation_request() and request.frame == page.main_frame:
            route.continue_()
        else:
            network.append({"url": request.url, "type": request.resource_type})
            route.abort()

    page.route("**/*", route_request)
    try:
        page.goto(url, wait_until="load")
        document_loaded = True
        page.locator("#replay-controls").wait_for(state="visible", timeout=15000)
        require(page.locator("#replay-controls").evaluate("el => Boolean(el.compareDocumentPosition(document.querySelector('header.hero')) & Node.DOCUMENT_POSITION_FOLLOWING)"),
                "Profile controls must precede the hero")
        require(page.locator('[data-profile-select="leaky"]').get_attribute("aria-pressed") == "true",
                "Leaky run must be selected by default")
        require(page.locator("#replay-status").get_attribute("role") == "status", "Missing accessible status role")
        require(page.locator("#replay-status").get_attribute("aria-live") == "polite", "Missing polite announcements")
        for profile in ("leaky", "fixed"):
            button = page.locator(f'[data-profile-select="{profile}"]')
            button.focus()
            page.keyboard.press("Space")
            require(button.get_attribute("aria-pressed") == "true", "Keyboard profile selection failed")
            require(button.evaluate("el => document.activeElement === el"), "Profile change moved keyboard focus")
            expected_count = sum(item["leaked"] for item in report[profile]["findings"])
            require(page.locator("#selected-findings").inner_text() == str(expected_count), "Selected finding count is stale")
            require(page.locator("#selected-rpc-count").inner_text() == f"{report[profile]['observation_count']} captured calls",
                    "Selected RPC count is stale")
            require(profile.capitalize() in page.locator("#replay-status").inner_text(), "Profile change was not announced")
            rpc_lines = source_lines(files[f"{profile}-observations.jsonl"])
            app_lines = source_lines(files[f"{profile}-app.log"])
            rpc = [json.loads(line) for line in rpc_lines]
            for finding in report[profile]["findings"]:
                check = finding["id"]
                panel = page.locator(f'#{check.lower()} .evidence-panel[data-profile="{profile}"]')
                other = "fixed" if profile == "leaky" else "leaky"
                require(panel.is_visible(), f"{check} did not switch to {profile}")
                require(not page.locator(f'#{check.lower()} .evidence-panel[data-profile="{other}"]').is_visible(),
                        f"{check} retained the other profile")
                require(panel.locator(".state").inner_text() == ("LEAK OBSERVED" if finding["leaked"] else "NO FINDING"),
                        f"{check} badge contradicts captured results")
                if check == "A3":
                    primary = panel.locator('.replay-stream[data-source="app"]')
                    check_stream(page, primary, app_lines, "app", finding["inference"])
                    separate = panel.locator("details.source-context")
                    require(separate.count() == 1, "A3 needs separate, labeled RPC context")
                    require("no shared clock" in primary.inner_text(), "A3 must not invent a shared RPC timestamp")
                    separate.locator(":scope > summary").click()
                    check_stream(page, separate.locator('.replay-stream[data-source="rpc"]'), rpc_lines, "rpc", finding["inference"])
                    separate.locator(":scope > summary").click()
                    raw_log = primary.locator("pre.raw-record").all_text_contents()
                    require(any("REDACTED" in line for line in raw_log) == (profile == "fixed"), "A3 log view is incorrect")
                else:
                    primary = panel.locator('.replay-stream[data-source="rpc"]')
                    check_stream(page, primary, rpc_lines, "rpc", finding["inference"])
                marked = [int(value) for value in primary.locator('[data-highlight="true"]').evaluate_all("els => els.map(el => el.dataset.seq)")]
                if check == "A1":
                    payment = [item for item in rpc if item["method"] == "GetTransaction" and item["args"]["txid"] == report["ground_truth"]["txid"]]
                    if profile == "fixed":
                        require(len(payment) == 1 and not marked, "Fixed payment fetch is legitimate public-window cover")
                        require(primary.locator(f'[data-seq="{payment[0]["seq"]}"]').is_visible(), "Fixed payment fetch disappeared")
                    else:
                        require(len(payment) == 2 and marked == [payment[0]["seq"]], "A1 must distinguish the extra lookup from public-window retrieval")
                if check == "A2":
                    sends = [item for item in rpc if item["method"] == "SendTransaction"]
                    require(len(sends) == (1 if profile == "leaky" else 0), "A2 server view invented or omitted a send")
                    require(marked == [item["seq"] for item in sends], "A2 highlight does not identify the actual send")
                    local = panel.locator(".local-evidence")
                    attempt = json.loads(files[f"{profile}-wallet.json"])["send_attempt"]
                    require(json.loads(local.locator("pre").inner_text()) == attempt, "A2 local evidence changed")
                    require("refusal is not an RPC event" in local.inner_text(), "A2 refusal must stay outside the server timeline")
            if screenshots:
                button.click()
                page.evaluate("window.scrollTo(0, 0)")
                page.screenshot(path=str(args.screenshots / f"{args.screenshot_prefix}-{profile}-top.png"))
                page.locator("#a1").evaluate("el => window.scrollTo(0, el.getBoundingClientRect().top + window.scrollY - 24)")
                page.screenshot(path=str(args.screenshots / f"{args.screenshot_prefix}-{profile}-a1.png"))
        result["checks"].extend(["default leaky view", "keyboard profile selection and focus", "all counts and cards switch",
                                  "exact original JSONL drawers", "real sequence numbers and manual stepping", "polite live announcements",
                                  "A1 extra versus legitimate payment fetch", "A3 separate application/RPC streams", "A2 local refusal versus observed send"])
        # Native details must open/close with the keyboard and copy exact source text.
        page.locator('[data-profile-select="leaky"]').click()
        first = page.locator('#a1 [data-profile="leaky"] .replay-row').first
        summary = first.locator(":scope > summary")
        summary.focus()
        page.keyboard.press("Enter")
        require(first.get_attribute("open") is not None, "Keyboard cannot open the original evidence")
        first.locator('[data-action="copy"]').click()
        expect(page.locator("#replay-status")).to_have_text(re.compile(r"^Copied "))
        require(page.evaluate("navigator.clipboard.readText()") == source_lines(files["leaky-observations.jsonl"])[0],
                "Copy record changed the source text")
        summary.focus()
        page.keyboard.press("Space")
        require(first.get_attribute("open") is None, "Keyboard cannot close the evidence drawer")
        for name, original in files.items():
            link = page.locator(f'.downloads a[download="{name}"]')
            require(link.get_attribute("href").startswith("blob:"), "Interactive download must use embedded evidence")
            with page.expect_download() as pending:
                link.click()
            require(Path(pending.value.path()).read_bytes() == original, f"Browser download changed bytes: {name}")
        result["checks"].extend(["keyboard raw drawers and exact clipboard copy", "all six browser downloads are byte-identical"])
        page.set_viewport_size({"width": 390, "height": 844})
        for profile in ("leaky", "fixed"):
            page.locator(f'[data-profile-select="{profile}"]').click()
            for selector in ("header.hero", "#a1", "#a3", "#a2"):
                page.locator(selector).evaluate("el => el.scrollIntoView()")
                require(page.evaluate("document.documentElement.scrollWidth <= innerWidth + 1"),
                        f"Horizontal mobile overflow in {profile} {selector}")
            row = page.locator(f'#a2 [data-profile="{profile}"] .replay-row').last
            if row.get_attribute("open") is None:
                row.locator(":scope > summary").click()
            require(page.evaluate("document.documentElement.scrollWidth <= innerWidth + 1"), "Expanded raw bytes overflow on mobile")
            if screenshots:
                page.locator("#a1").evaluate("el => el.scrollIntoView()")
                page.screenshot(path=str(args.screenshots / f"{args.screenshot_prefix}-{profile}-mobile.png"))
        require(page.evaluate("matchMedia('(prefers-reduced-motion: reduce)').matches"), "Reduced motion was not applied")
        require(page.evaluate("getComputedStyle(document.documentElement).scrollBehavior") == "auto", "Reduced motion must disable smooth scrolling")
        violations.extend(page.evaluate("window.__reviewCsp"))
        require(not errors, f"Browser errors: {errors}")
        require(not violations, f"CSP violations: {violations}")
        require(not network, f"Browser attempted network requests after the document: {network}")
        result["checks"].extend(["390px mobile layout and expanded raw records", "reduced motion", "zero console errors or CSP violations", "zero subresource/network requests"])
    finally:
        context.close()
    static_context = browser.new_context(java_script_enabled=False, viewport={"width": 1440, "height": 1000})
    try:
        static = static_context.new_page()
        static.goto(url, wait_until="load")
        require(not static.locator("#replay-controls").is_visible(), "Inert replay controls remain visible without JavaScript")
        for check in ("a1", "a3", "a2"):
            for profile in ("leaky", "fixed"):
                panel = static.locator(f'#{check} .evidence-panel[data-profile="{profile}"]')
                require(panel.is_visible() and panel.locator(".static-trace").is_visible(), "JavaScript-disabled comparison is incomplete")
        require(static.locator("#paired-counts").is_visible(), "Static paired result is unavailable")
        result["checks"].append("JavaScript-disabled complete before/after comparison")
    finally:
        static_context.close()
    result["passed"] = True
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, default=ROOT / "report/index.html")
    parser.add_argument("--url", help="Verify only this deployed report URL")
    parser.add_argument("--expected-commit")
    parser.add_argument("--edge", type=Path)
    parser.add_argument("--screenshots", type=Path, default=ROOT / "docs/replay")
    parser.add_argument("--screenshot-prefix", default="after")
    parser.add_argument("--output", type=Path, default=ROOT / ".tools/replay-verification.json")
    args = parser.parse_args()
    from playwright.sync_api import sync_playwright

    edge = args.edge or Path(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe")
    args.screenshots.mkdir(parents=True, exist_ok=True)
    with sync_playwright() as playwright:
        options = {"headless": True}
        if edge.is_file():
            options["executable_path"] = str(edge)
        browser = playwright.chromium.launch(**options)
        try:
            if args.url:
                results = [check_browser(browser, args.url, args, screenshots=True)]
            else:
                path = args.report.resolve()
                results = [check_browser(browser, path.as_uri(), args, screenshots=True)]
                with local_server(path.parent) as url:
                    results.append(check_browser(browser, url, args, screenshots=False))
        finally:
            browser.close()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({"passed": True, "targets": results}, indent=2) + "\n",
                           encoding="utf-8", newline="\n")
    print(json.dumps({"passed": True, "targets": [result["url"] for result in results],
                      "evidence": str(args.output)}, indent=2))


if __name__ == "__main__":
    main()
