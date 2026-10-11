const tabs = document.querySelectorAll(".tab");
const panels = document.querySelectorAll(".panel");
const invoke = window.__TAURI__?.core?.invoke ?? null;

const els = {
  connectionDot: document.querySelector("#status-dot"),
  connectionLabel: document.querySelector("#connection-label"),
  sessionMessage: document.querySelector("#session-message"),
  runtimeNote: document.querySelector("#runtime-note"),
  start: document.querySelector("#start-session"),
  stop: document.querySelector("#stop-session"),
  reset: document.querySelector("#reset-session"),
  presentmon: document.querySelector("#presentmon-executable"),
  output: document.querySelector("#output-file"),
  pid: document.querySelector("#process-id"),
  duration: document.querySelector("#duration-seconds"),
  interval: document.querySelector("#sample-interval-ms"),
  excludeDropped: document.querySelector("#exclude-dropped"),
  noProcessProbing: document.querySelector("#no-process-probing"),
  resultTitle: document.querySelector("#result-title"),
  resultDetail: document.querySelector("#result-detail"),
  resultFrames: document.querySelector("#result-frames"),
  resultTelemetry: document.querySelector("#result-telemetry"),
  stutterCount: document.querySelector("#stutter-count"),
  stutterBody: document.querySelector("#stutter-body"),
  diagnosticCount: document.querySelector("#diagnostic-count"),
  diagnosticList: document.querySelector("#diagnostic-list"),
  compareP99: document.querySelector("#compare-p99"),
  compareFps: document.querySelector("#compare-fps"),
  compareStutters: document.querySelector("#compare-stutters"),
  refreshHistory: document.querySelector("#refresh-history"),
  historyList: document.querySelector("#history-list"),
  compareBaseline: document.querySelector("#compare-baseline"),
  compareCandidate: document.querySelector("#compare-candidate"),
  compareSameScenario: document.querySelector("#compare-same-scenario"),
  runCompare: document.querySelector("#run-compare"),
  processSelect: document.querySelector("#process-select"),
  refreshProcesses: document.querySelector("#refresh-processes"),
  browsePresentmon: document.querySelector("#browse-presentmon"),
  browseOutput: document.querySelector("#browse-output"),
  progress: document.querySelector("#capture-progress"),
  progressFill: document.querySelector("#capture-progress-fill"),
  remediation: document.querySelector("#session-remediation"),
  stutterChart: document.querySelector("#stutter-chart"),
};

let pollTimer = null;

for (const tab of tabs) {
  tab.addEventListener("click", () => {
    const target = tab.dataset.tab;
    for (const candidate of tabs) {
      candidate.classList.toggle("active", candidate === tab);
    }
    for (const panel of panels) {
      panel.classList.toggle("active", panel.dataset.panel === target);
    }
  });
}

function setConnection(label, running) {
  els.connectionLabel.textContent = label;
  els.connectionDot.classList.toggle("live", Boolean(running));
}

function setControls(running) {
  els.start.disabled = running;
  els.stop.disabled = !running;
  els.reset.disabled = running;
}

function setMessage(message) {
  els.sessionMessage.textContent = message;
}

async function showFailure(message) {
  setMessage(message);
  if (!invoke || !els.remediation) return;
  try {
    const hint = await invoke("session_error_remediation", { message: String(message) });
    if (hint) {
      els.remediation.textContent = `Fix: ${hint}`;
      els.remediation.style.display = "";
      return;
    }
  } catch (error) {
    /* remediation is best-effort; the raw message above stands alone */
  }
  els.remediation.style.display = "none";
}

function clearRemediation() {
  if (els.remediation) els.remediation.style.display = "none";
}

function setProgress(running, snapshot) {
  if (!els.progress || !els.progressFill) return;
  if (!running || !snapshot || typeof snapshot.started_unix_ms !== "number" || !snapshot.duration_seconds) {
    els.progress.style.display = "none";
    return;
  }
  const elapsed = Math.max(0, Date.now() - snapshot.started_unix_ms) / 1000;
  const ratio = Math.min(1, elapsed / snapshot.duration_seconds);
  els.progress.style.display = "";
  els.progressFill.style.width = `${(ratio * 100).toFixed(1)}%`;
}

function drawStutterChart(stutters) {
  const canvas = els.stutterChart;
  if (!canvas || typeof canvas.getContext !== "function") return;
  const context = canvas.getContext("2d");
  if (!context) return;
  const width = canvas.width;
  const height = canvas.height;
  context.clearRect(0, 0, width, height);
  if (!stutters.length) return;
  const peak = Math.max(...stutters.map((row) => row.peak_frame_time_ms), 1);
  const barWidth = width / stutters.length;
  stutters.forEach((row, index) => {
    const barHeight = (row.peak_frame_time_ms / peak) * (height - 8);
    const baselineHeight = (row.baseline_frame_time_ms / peak) * (height - 8);
    context.fillStyle = "#4caf50";
    context.fillRect(index * barWidth + 1, height - barHeight, barWidth - 2, barHeight);
    context.fillStyle = "#ffffff";
    context.fillRect(index * barWidth + 1, height - baselineHeight, barWidth - 2, 2);
  });
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function metricCard(metric) {
  const card = document.querySelector(`[data-metric="${metric.id}"]`);
  if (!card) return;
  card.querySelector("span").textContent = metric.title.toUpperCase();
  card.querySelector("strong").textContent = metric.value;
  card.querySelector("small").textContent = metric.detail;
}

function renderCompleted(snapshot) {
  const dashboard = snapshot.dashboard;
  for (const metric of dashboard.metrics) {
    metricCard(metric);
  }

  els.resultTitle.textContent = `${snapshot.application} · PID ${snapshot.process_id}`;
  els.resultDetail.textContent = "Capture completed. Analysis and diagnostics are deterministic for this session.";
  els.resultFrames.textContent = `${snapshot.frame_count} frames`;
  els.resultTelemetry.textContent = `${snapshot.system_sample_count} telemetry samples`;

  const stutters = dashboard.stutters ?? [];
  drawStutterChart(stutters);
  els.stutterCount.textContent = `${stutters.length} event${stutters.length === 1 ? "" : "s"}`;
  els.stutterBody.innerHTML = stutters.length
    ? stutters.map((row) => `
        <tr>
          <td>${row.start_ms.toFixed(2)} ms</td>
          <td>${row.duration_ms.toFixed(2)} ms</td>
          <td>${row.peak_frame_time_ms.toFixed(2)} ms</td>
          <td>${row.baseline_frame_time_ms.toFixed(2)} ms</td>
          <td>${row.candidate_frame_count}</td>
        </tr>`).join("")
    : '<tr><td colspan="5" class="empty-row">No stutter events detected</td></tr>';

  const diagnostics = dashboard.diagnostics ?? [];
  els.diagnosticCount.textContent = `${diagnostics.length} diagnosis${diagnostics.length === 1 ? "" : "es"}`;
  els.diagnosticList.innerHTML = diagnostics.length
    ? diagnostics.map((item) => `
      <article class="diagnostic-card">
        <div class="diagnostic-score">${item.evidence_score}<span>/100</span></div>
        <div class="diagnostic-content">
          <div class="diagnostic-title">
            <div>
              <span class="label">${escapeHtml(item.category)}</span>
              <h3>${escapeHtml(item.rule_id)}</h3>
            </div>
          </div>
          ${item.supporting_evidence.length
            ? `<div class="evidence-block"><span class="evidence-label">Supporting</span><ul>${item.supporting_evidence.map((e) => `<li>${escapeHtml(e)}</li>`).join("")}</ul></div>`
            : ""}
          ${item.contradicting_evidence.length
            ? `<div class="evidence-block"><span class="evidence-label">Contradicting</span><ul>${item.contradicting_evidence.map((e) => `<li>${escapeHtml(e)}</li>`).join("")}</ul></div>`
            : ""}
        </div>
      </article>`).join("")
    : `<div class="diagnostic-placeholder"><div class="score">—<span>/100</span></div><div><h3>Unknown</h3><p>Insufficient evidence.</p></div></div>`;

  els.compareP99.textContent = "—";
  els.compareFps.textContent = "—";
  els.compareStutters.textContent = "—";
  setConnection("CAPTURE COMPLETE", false);
  setMessage(`${snapshot.application} · ${snapshot.frame_count} frames`);
  setControls(false);
}

async function refreshHistory() {
  if (!invoke) return;
  try {
    const entries = await invoke("session_history");
    els.historyList.innerHTML = entries.length
      ? entries.map((entry) => `
        <article class="diagnostic-card">
          <div class="diagnostic-content">
            <div class="diagnostic-title">
              <div>
                <span class="label">${entry.verified ? "VERIFIED" : "INVALID"}</span>
                <h3>${escapeHtml(entry.application)} · ${escapeHtml(entry.session_id)}</h3>
              </div>
              <button class="ghost" data-load-session="${escapeHtml(entry.session_id)}">Load</button>
            </div>
            <p>${entry.frame_count} frames · PID ${entry.process_id}${entry.interrupted ? " · interrupted" : ""}</p>
          </div>
        </article>`).join("")
      : '<div class="diagnostic-placeholder"><div><h3>No persisted sessions</h3><p>Completed GUI captures appear here after manifest verification.</p></div></div>';
    for (const select of [els.compareBaseline, els.compareCandidate]) {
      if (!select) continue;
      const current = select.value;
      select.innerHTML = '<option value="">Select a verified session</option>' +
        entries.map((entry) =>
          `<option value="${escapeHtml(entry.session_id)}">${escapeHtml(entry.application)} · ${escapeHtml(entry.session_id)}</option>`
        ).join("");
      if (entries.some((entry) => entry.session_id === current)) select.value = current;
    }

    els.historyList.querySelectorAll("[data-load-session]").forEach((button) => {
      button.addEventListener("click", async () => {
        try {
          const snapshot = await invoke("session_history_load", { sessionId: button.dataset.loadSession });
          renderSnapshot(snapshot);
        } catch (error) {
          setMessage(String(error));
        }
      });
    });
  } catch (error) {
    els.historyList.textContent = String(error);
  }
}

function renderSnapshot(snapshot) {
  if (snapshot.status === "idle") {
    setConnection("LOCAL SESSION", false);
    setMessage("Ready");
    clearRemediation();
    setProgress(false);
    setControls(false);
    return;
  }

  if (snapshot.status === "running") {
    setConnection("CAPTURING", true);
    setMessage(`PID ${snapshot.process_id} · collecting telemetry`);
    clearRemediation();
    setProgress(true, snapshot);
    setControls(true);
    return;
  }

  if (snapshot.status === "failed") {
    setConnection("SESSION ERROR", false);
    setProgress(false);
    showFailure(snapshot.message);
    els.resultTitle.textContent = "Capture failed";
    els.resultDetail.textContent = snapshot.message;
    setControls(false);
    return;
  }

  if (snapshot.status === "completed") {
    setProgress(false);
    clearRemediation();
    renderCompleted(snapshot);
  }
}

async function refreshStatus() {
  if (!invoke) {
    setConnection("BROWSER PREVIEW", false);
    setMessage("Tauri runtime unavailable");
    els.runtimeNote.textContent = "This page is presentation-only outside the Tauri application.";
    return;
  }

  try {
    const snapshot = await invoke("session_status");
    renderSnapshot(snapshot);
    if (snapshot.status !== "running" && pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  } catch (error) {
    setConnection("IPC ERROR", false);
    setMessage(String(error));
  }
}

function startPolling() {
  if (pollTimer) return;
  pollTimer = setInterval(refreshStatus, 500);
}

els.start.addEventListener("click", async () => {
  if (!invoke) {
    setMessage("Start is only available inside the native Tauri application.");
    return;
  }

  const processId = Number(els.pid.value);
  const duration = Number(els.duration.value);
  const interval = Number(els.interval.value);

  if (!els.presentmon.value.trim() || !els.output.value.trim() || !Number.isInteger(processId) || processId <= 0) {
    setMessage("PresentMon path, output path, and a valid target PID are required.");
    return;
  }

  if (!Number.isInteger(duration) || duration < 1 || duration > 3600) {
    setMessage("Duration must be between 1 and 3600 seconds.");
    return;
  }

  if (!Number.isInteger(interval) || interval < 10 || interval > 5000) {
    setMessage("Telemetry interval must be between 10 and 5000 ms.");
    return;
  }

  try {
    const snapshot = await invoke("session_start", {
      request: {
        presentmon_executable: els.presentmon.value.trim(),
        output_file: els.output.value.trim(),
        process_id: processId,
        duration_seconds: duration,
        sample_interval_ms: interval,
        exclude_dropped: els.excludeDropped.checked,
        no_process_probing: els.noProcessProbing.checked,
      },
    });
    renderSnapshot(snapshot);
    startPolling();
  } catch (error) {
    showFailure(String(error));
  }
});

els.stop.addEventListener("click", async () => {
  if (!invoke) return;
  try {
    const snapshot = await invoke("session_stop");
    renderSnapshot(snapshot);
    startPolling();
  } catch (error) {
    setMessage(String(error));
  }
});

els.refreshHistory?.addEventListener("click", refreshHistory);

async function refreshProcesses() {
  if (!invoke || !els.processSelect) return;
  try {
    const entries = await invoke("session_processes");
    const current = els.pid.value;
    els.processSelect.innerHTML = '<option value="">Select a live process</option>' +
      entries.map((entry) =>
        `<option value="${entry.process_id}">${escapeHtml(entry.name)} · PID ${entry.process_id}</option>`
      ).join("");
    if (current) els.pid.value = current;
  } catch (error) {
    setMessage(String(error));
  }
}

if (els.processSelect) {
  refreshProcesses();
  els.processSelect.addEventListener("change", () => {
    if (els.processSelect.value) els.pid.value = els.processSelect.value;
  });
}

els.refreshProcesses?.addEventListener("click", refreshProcesses);

els.browsePresentmon?.addEventListener("click", async () => {
  if (!invoke) return;
  try {
    const picked = await invoke("session_pick_presentmon_file");
    if (picked) els.presentmon.value = picked;
  } catch (error) {
    setMessage(String(error));
  }
});

els.browseOutput?.addEventListener("click", async () => {
  if (!invoke) return;
  try {
    const picked = await invoke("session_pick_output_file");
    if (picked) els.output.value = picked;
  } catch (error) {
    setMessage(String(error));
  }
});

function listenSessionEvents() {
  const listen = window.__TAURI__?.event?.listen;
  if (typeof listen !== "function") return;
  listen("rwp-session-event", (event) => {
    const payload = event?.payload;
    if (!payload || typeof payload.event !== "string") return;
    if (payload.event === "started") {
      startPolling();
    } else {
      refreshStatus();
    }
  }).catch(() => {});
}

listenSessionEvents();

els.runCompare?.addEventListener("click", async () => {
  if (!invoke) {
    setMessage("Compare is only available inside the native Tauri application.");
    return;
  }
  const baselineSessionId = els.compareBaseline?.value;
  const candidateSessionId = els.compareCandidate?.value;
  if (!baselineSessionId || !candidateSessionId) {
    setMessage("Select both a baseline and candidate session.");
    return;
  }
  if (baselineSessionId === candidateSessionId) {
    setMessage("Baseline and candidate must be different sessions.");
    return;
  }
  try {
    const comparison = await invoke("session_compare", {
      baselineSessionId,
      candidateSessionId,
      sameScenario: Boolean(els.compareSameScenario?.checked),
    });
    els.compareP99.textContent = comparison.p99_change;
    els.compareFps.textContent = comparison.average_fps_change;
    els.compareStutters.textContent = comparison.stutter_change;
    setMessage(`Compare: ${comparison.comparability}`);
  } catch (error) {
    setMessage(String(error));
  }
});

els.reset.addEventListener("click", async () => {
  if (!invoke) return;
  try {
    const snapshot = await invoke("session_reset");
    renderSnapshot(snapshot);
  } catch (error) {
    setMessage(String(error));
  }
});

refreshStatus();
refreshHistory();
