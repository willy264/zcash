(async () => {
  "use strict";
  const dataNode = document.getElementById("replay-data");
  if (!dataNode) return;
  const controls = document.getElementById("replay-controls");
  const status = document.getElementById("replay-status");
  const profiles = ["leaky", "fixed"];
  const streams = [];
  const blobUrls = [];
  const el = (tag, className, text) => {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  };
  const announce = (message) => { status.textContent = message; };
  const short = (value) => value.length > 28 ? value.slice(0, 12) + "…" + value.slice(-8) : value;
  const values = (value) => typeof value === "string" ? [value]
    : value && typeof value === "object" ? Object.values(value).flatMap(values) : [];
  const jsonLines = (text) => {
    const lines = text.split("\n");
    if (lines[lines.length - 1] === "") lines.pop();
    if (!lines.length || lines.some((line) => !line.trim())) throw new Error("Missing captured records");
    return lines.map((line, index) => ({ raw: line.replace(/\r$/, ""), record: JSON.parse(line), line: index + 1 }));
  };

  try {
    const data = JSON.parse(dataNode.textContent);
    if (data.schema_version !== 1 || !data.report || !data.files) throw new Error("Unsupported evidence");
    const report = data.report;
    const decoded = {};
    // Decode the exact captured bytes, including original line endings. Validate
    // every file before hiding any of the complete static comparison.
    for (const profile of profiles) {
      for (const suffix of ["observations.jsonl", "app.log", "wallet.json"]) {
        const name = profile + "-" + suffix;
        const file = data.files[name];
        if (!file || !Number.isSafeInteger(file.byte_length)) throw new Error("Missing captured file");
        const bytes = Uint8Array.from(atob(file.base64), (c) => c.charCodeAt(0));
        if (bytes.length !== file.byte_length) throw new Error("Captured file length changed");
        const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
        const hash = Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
        if (hash !== file.sha256) throw new Error("Captured file hash changed");
        decoded[name] = { bytes, text: new TextDecoder("utf-8", { fatal: true }).decode(bytes) };
      }
    }

    function rpcSummary(row) {
      const args = row.record.args || {};
      let detail = "";
      if (row.record.method === "GetTransaction" && typeof args.txid === "string") detail = short(args.txid);
      else if (row.record.method === "GetBlockRange") detail = String(args.start) + "–" + String(args.end);
      else if (row.record.method === "SendTransaction") detail = String(args.bytes) + " captured bytes";
      return row.record.method + (detail ? "(" + detail + ")" : "");
    }

    function makeStream(profile, check, source, rows, marked, extraSeq, inference) {
      const stream = el("section", "replay-stream");
      stream.dataset.source = source;
      const sourceName = profile + (source === "rpc" ? "-observations.jsonl" : "-app.log");
      const kind = source === "rpc" ? "RPC records" : "application log lines";
      stream.setAttribute("aria-label", check + " " + profile + " " + kind);
      stream.append(el("h4", "", source === "rpc" ? "Captured server view / observations.jsonl" : "Captured application view / app.log"));
      stream.append(el("p", "stream-note", source === "rpc"
        ? "Actual request order and sequence numbers. Expand a record to inspect its exact JSONL line."
        : "Original log-file order. This stream has no shared clock with the RPC trace."));
      const stepControls = el("div", "step-controls");
      const progress = el("span", "step-count");
      const buttons = {};
      for (const [action, label] of [["restart", "Restart"], ["next", "Next"], ["all", "Show all"]]) {
        const button = el("button", "", label);
        button.type = "button";
        button.dataset.action = action;
        button.setAttribute("aria-label", label + " " + check + " " + kind);
        buttons[action] = button;
        stepControls.append(button);
      }
      stepControls.append(progress);
      stream.append(stepControls);
      const elements = rows.map((row) => {
        const seq = source === "rpc" ? row.record.seq : row.line;
        const item = el("details", "replay-row");
        item.dataset.seq = String(seq);
        item.dataset.highlight = String(marked.has(seq));
        item.open = marked.has(seq);
        const summary = el("summary");
        summary.append(el("span", "row-seq", source === "rpc" ? "#" + seq : "line " + seq));
        summary.append(el("span", "row-method", source === "rpc" ? rpcSummary(row) : String(row.record.event || "Application event")));
        let tag = "Inspect source";
        if (source === "rpc" && seq === extraSeq) tag = "Extra payment lookup";
        else if (marked.has(seq)) tag = "Finding evidence";
        else if (source === "rpc" && row.record.args && row.record.args.txid === report.ground_truth.txid) tag = "Fixture payment · ground-truth label";
        summary.append(el("span", "row-tag", tag));
        item.append(summary, el("pre", "raw-record", row.raw));
        if (marked.has(seq)) item.append(el("p", "record-inference", inference));
        item.append(el("p", "source-line", sourceName + " · line " + row.line + " · original captured record"));
        const copyControls = el("div", "step-controls");
        const copy = el("button", "", "Copy record");
        copy.type = "button";
        copy.dataset.action = "copy";
        copy.setAttribute("aria-label", "Copy " + check + " " + kind + " " + seq);
        copy.addEventListener("click", async () => {
          try {
            await navigator.clipboard.writeText(row.raw);
            announce("Copied " + sourceName + " line " + row.line + ".");
          } catch (_) {
            announce("Clipboard access is unavailable. Select the original record text above to copy it.");
          }
        });
        copyControls.append(copy);
        item.append(copyControls);
        stream.append(item);
        if (source === "app") {
          // A visible decoded preview lets the disclosure be read immediately;
          // the expandable row and download retain the exact original bytes.
          const preview = el("pre", "terminal app-preview", JSON.stringify(row.record, null, 2));
          preview.dataset.line = String(row.line);
          preview.setAttribute("aria-label", "Decoded application record " + row.line);
          stream.append(preview);
          item.preview = preview;
        }
        return item;
      });
      const empty = el("p", "empty-stream", rows.length ? "No records revealed. Select Next or Show all." : "No captured records in this stream.");
      stream.append(empty);
      let shown = rows.length;
      function show(count, speak, openNewest = false) {
        shown = Math.max(0, Math.min(count, rows.length));
        elements.forEach((item, index) => {
          item.hidden = index >= shown;
          if (item.preview) item.preview.hidden = item.hidden;
          if (!shown) item.open = false;
          else if (shown === rows.length && item.dataset.highlight === "true") item.open = true;
        });
        const latest = elements[shown - 1];
        if (openNewest && latest) latest.open = true;
        empty.hidden = shown !== 0;
        buttons.restart.disabled = shown === 0;
        buttons.next.disabled = shown === rows.length;
        buttons.all.disabled = shown === rows.length;
        progress.textContent = shown + " / " + rows.length + " " + kind;
        if (speak) {
          const hit = openNewest && latest && latest.dataset.highlight === "true";
          announce(check + " " + profile + ": " + shown + " of " + rows.length + " " + kind + " shown. " + (hit ? "Finding evidence. " + inference : "Full-run findings are unchanged."));
        }
      }
      buttons.restart.addEventListener("click", () => show(0, true));
      buttons.next.addEventListener("click", () => show(shown + 1, true, true));
      buttons.all.addEventListener("click", () => show(rows.length, true));
      show(rows.length, false);
      streams.push({ profile, all: () => show(rows.length, false) });
      return stream;
    }

    const preparations = [];
    for (const profile of profiles) {
      const run = report[profile];
      const rpcRows = jsonLines(decoded[profile + "-observations.jsonl"].text);
      const appRows = jsonLines(decoded[profile + "-app.log"].text);
      const wallet = JSON.parse(decoded[profile + "-wallet.json"].text);
      if (rpcRows.length !== run.observation_count || rpcRows.some((row, index) => row.record.seq !== index + 1)) throw new Error("Incomplete RPC trace");
      for (const panel of document.querySelectorAll('.evidence-panel[data-profile="' + profile + '"]')) {
        const check = panel.dataset.check;
        const finding = run.findings.find((item) => item.id === check);
        if (!finding) throw new Error("Missing checked finding");
        const evidence = el("div", "replay-evidence");
        evidence.append(el("p", "full-run-label", "Verdict above and explanation below cover the complete captured run, including records hidden while stepping."));
        let marked = new Set(finding.observation_sequences);
        let extraSeq;
        if (check === "A1" && finding.leaked) {
          const fetches = rpcRows.filter((row) => row.record.method === "GetTransaction");
          const extras = fetches.filter((row, index) => {
            const remaining = fetches.filter((_, i) => i !== index).map((item) => item.record.args.txid);
            return row.record.args.txid === report.ground_truth.txid && JSON.stringify(remaining) === JSON.stringify(report.candidate_txids);
          });
          if (extras.length === 1) { extraSeq = extras[0].record.seq; marked = new Set([extraSeq]); }
        }
        if (check === "A3") {
          const privateValues = [report.ground_truth.memo_plaintext, ...(wallet.notes || []).map((note) => note.recipient_addr)].filter(Boolean);
          const appMarked = new Set(appRows.filter((row) => finding.leaked && values(row.record).some((value) => privateValues.some((secret) => value.includes(secret)))).map((row) => row.line));
          evidence.append(makeStream(profile, check, "app", appRows, appMarked, undefined, finding.inference));
          const context = el("details", "source-context");
          context.append(el("summary", "", "Inspect the separate RPC context (" + rpcRows.length + " captured calls)"));
          context.append(makeStream(profile, check, "rpc", rpcRows, new Set(), undefined, finding.inference));
          evidence.append(context);
        } else {
          evidence.append(makeStream(profile, check, "rpc", rpcRows, marked, extraSeq, finding.inference));
          if (check === "A2") {
            const local = el("section", "local-evidence");
            local.append(el("h4", "", "Local subject evidence / wallet.json"));
            local.append(el("p", "stream-note", "A local decision, separate from the server trace. A refusal is not an RPC event. Unsigned test bytes do not establish acceptance or move funds."));
            local.append(el("pre", "", JSON.stringify(wallet.send_attempt, null, 2)));
            local.append(el("p", "source-line", profile + "-wallet.json · send_attempt field · " + rpcRows.filter((row) => row.record.method === "SendTransaction").length + " captured SendTransaction calls"));
            evidence.append(local);
          }
        }
        preparations.push({ panel, evidence });
      }
    }

    // All reads, hashes and construction have succeeded. Enhance atomically;
    // ordinary links and both static panels remain useful if any step fails.
    for (const [name, file] of Object.entries(decoded)) {
      const url = URL.createObjectURL(new Blob([file.bytes], { type: "application/octet-stream" }));
      blobUrls.push(url);
      for (const link of document.querySelectorAll('a[download][href="' + name + '"]')) {
        link.href = url;
        link.download = name;
      }
    }
    for (const { panel, evidence } of preparations) {
      panel.querySelector(".static-trace").hidden = true;
      panel.insertBefore(evidence, panel.querySelector(".explanation"));
    }
    function select(profile) {
      const run = report[profile];
      for (const panel of document.querySelectorAll(".evidence-panel[data-profile]")) panel.hidden = panel.dataset.profile !== profile;
      for (const button of document.querySelectorAll("[data-profile-select]")) button.setAttribute("aria-pressed", String(button.dataset.profileSelect === profile));
      streams.filter((stream) => stream.profile === profile).forEach((stream) => stream.all());
      const count = run.findings.filter((finding) => finding.leaked).length;
      const findings = document.getElementById("selected-findings");
      findings.textContent = String(count);
      findings.className = "count " + (count ? "bad" : "good");
      document.getElementById("selected-profile-label").textContent = profile.toUpperCase() + " RUN / FULL-RUN FINDINGS";
      document.getElementById("selected-rpc-count").textContent = run.observation_count + " captured calls";
      document.getElementById("rpc-count-label").textContent = profile.toUpperCase() + " RUN OBSERVATIONS";
      document.body.dataset.selectedProfile = profile;
      announce((profile === "leaky" ? "Leaky" : "Fixed") + " run: " + count + " findings, " + run.observation_count + " captured calls. All records shown.");
    }
    for (const button of document.querySelectorAll("[data-profile-select]")) button.addEventListener("click", () => select(button.dataset.profileSelect));
    document.getElementById("paired-counts").hidden = true;
    document.getElementById("selected-result").hidden = false;
    document.body.classList.add("replay-ready");
    controls.hidden = false;
    select("leaky");
    window.addEventListener("pagehide", (event) => {
      if (!event.persisted) blobUrls.forEach((url) => URL.revokeObjectURL(url));
    });
  } catch (error) {
    // Fail back to the complete static before/after report; never show a
    // partially decoded or invented run as interactive evidence.
    const note = el("p", "boundary", "Replay is unavailable in this browser or its embedded evidence failed validation. The complete captured before/after report remains below.");
    document.getElementById("evidence").prepend(note);
  }
})();
