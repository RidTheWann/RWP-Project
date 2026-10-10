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
    setControls(false);
    return;
  }

  if (snapshot.status === "running") {
    setConnection("CAPTURING", true);
    setMessage(`PID ${snapshot.process_id} · collecting telemetry`);
    setControls(true);
    return;
  }

  if (snapshot.status === "failed") {
    setConnection("SESSION ERROR", false);
    setMessage(snapshot.message);
    els.resultTitle.textContent = "Capture failed";
    els.resultDetail.textContent = snapshot.message;
    setControls(false);
    return;
  }

  if (snapshot.status === "completed") {
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
    setMessage(String(error));
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
