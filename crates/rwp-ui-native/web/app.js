const invoke = window.__TAURI__ && window.__TAURI__.core ? window.__TAURI__.core.invoke : null;
const tauriEvent = window.__TAURI__ && window.__TAURI__.event ? window.__TAURI__.event : null;

const STRINGS = {
  id: {
    navHome:"Beranda",navHistory:"Riwayat",navSettings:"Pengaturan",homeEyebrow:"PERFORMA GAME",homeTitle:"Ukur performa game Anda",homeSubtitle:"Lihat hasil pengukuran performa game di PC Anda.",
    gameLabel:"Pilih game",gameHint:"Pilih aplikasi yang ingin Anda ukur.",refresh:"Segarkan",runningApps:"Aplikasi yang sedang berjalan",chooseApp:"Pilih aplikasi",gameEmpty:"Buka game Anda dulu, lalu Segarkan.",
    readinessTitle:"Kesiapan pengukuran",permissionLabel:"Izin pengukuran",permissionReady:"Siap",permissionNeeded:"Perlu diperbaiki",componentLabel:"Komponen pengukuran",componentReady:"Siap",componentNeeded:"Perlu disiapkan",fixIt:"Lihat cara memperbaiki",setupComponent:"Siapkan komponen",readinessNote:"RWP akan memeriksa ulang sebelum mulai mengukur.",
    quickOptions:"Pilihan pengukuran",prepLabel:"Waktu bersiap",prep5:"5 detik",prep10:"10 detik",prep15:"15 detik",notifyStable:"Beri tahu saya jika hasilnya stabil",notifyStableHint:"Notifikasi hanya muncul setelah pengukuran selesai.",startMeasure:"Mulai ukur",ready:"Siap mengukur",checking:"Memeriksa kesiapan…",
    noProcess:"Belum ada aplikasi yang terdeteksi. Buka game Anda, lalu Segarkan.",refreshFailed:"Daftar aplikasi tidak dapat diperbarui.",preflightFailed:"Kesiapan pengukuran tidak dapat diperiksa. Coba lagi.",countdownEyebrow:"BERSIAP",countdownTitle:"Bersiap mengukur",countdownBody:"Pindah ke game Anda.",countdownHint:"Pengukuran dimulai setelah hitung mundur.",countdownAnnouncement:"Pindah ke game Anda. Pengukuran dimulai dalam {seconds} detik.",cancel:"Batalkan",
    measureEyebrow:"PENGUKURAN BERLANGSUNG",recordingTitle:"Sedang merekam",recordingBody:"Sedang merekam. Mainkan seperti biasa; hasil muncul setelah selesai.",recordingFooter:"Hasil akan muncul setelah pengukuran selesai.",stopMeasure:"Hentikan pengukuran",analyzingEyebrow:"TAHAP AKHIR",analyzingTitle:"Menganalisis…",analyzingBody:"Menyiapkan hasil pengukuran Anda.",analyzingHint:"RWP sedang menyelesaikan hasil. Tombol berhenti dinonaktifkan.",
    resultEyebrow:"HASIL PENGUKURAN",resultHeading:"Hasil pengukuran",reasonLabel:"Alasan",adviceLabel:"Saran berikutnya",alsoDetected:"Juga terdeteksi",expertDetails:"Detail untuk ahli",measureAgain:"Ukur lagi",compare:"Bandingkan",compareDisabled:"Belum ada pengukuran lain dari game yang sama untuk dibandingkan.",
    stabilityUnknown:"Stabilitas belum diketahui",stabilityUnknownBody:"Belum ada bukti yang cukup untuk menetapkan status stabilitas.",stabilityStable:"Stabil",stabilityStableBody:"Pengukuran memenuhi kriteria stabilitas yang ada. Ini bukan jaminan bahwa game selalu lancar.",stabilityUnstable:"Tidak stabil",stabilityUnstableBody:"Ada peristiwa stutter dalam data pengukuran yang tersedia.",observations:"Jumlah observasi: {count}",stutters:"Peristiwa stutter: {count}",
    historyEyebrow:"HASIL TERSIMPAN LOKAL",historyTitle:"Riwayat pengukuran",historySubtitle:"Pilih pengukuran untuk melihat hasil atau membandingkannya.",savedMeasurements:"Pengukuran tersimpan",historyEmptyTitle:"Belum ada riwayat pengukuran",historyEmptyBody:"Pengukuran yang tersimpan akan muncul di sini.",measureGame:"Ukur game",
    compareTitle:"Sebelum / Sesudah",compareHint:"Pilih sesi dari daftar dengan tombol Sebelum atau Sesudah.",beforeLabel:"Sebelum",afterLabel:"Sesudah",notSelected:"Belum dipilih",scenarioQuestion:"Apakah kondisi kedua pengukuran cukup serupa?",scenarioYes:"Ya, kondisinya sebanding",scenarioUnsure:"Tidak yakin",scenarioNo:"Tidak, kondisinya berbeda",compareAction:"Bandingkan hasil",compareSelectHint:"Pilih dua pengukuran terverifikasi dari aplikasi yang sama.",scenarioRequired:"Pilih jawaban tentang kondisi pengukuran.",
    compareComparable:"Hasil dapat dibandingkan",compareComparableBody:"Kedua pengukuran memenuhi kriteria perbandingan yang tersedia.",compareLimited:"Perbandingan terbatas",compareLimitedBody:"Jumlah data pada kedua pengukuran berbeda. Bandingkan hasil dengan hati-hati.",compareInconclusive:"Belum bisa disimpulkan",compareInconclusiveBody:"Kondisi kedua pengukuran tidak dapat dipastikan sebanding.",compareAdvice:"Ulangi pengukuran dengan kondisi yang serupa agar hasil lebih mudah dibandingkan.",sameApplication:"Pilih dua pengukuran dari aplikasi yang sama.",verifiedRequired:"Data sesi tidak terverifikasi dan tidak dapat dipilih.",compareFailed:"Perbandingan tidak dapat diselesaikan.",
    settingsEyebrow:"PREFERENSI LOKAL",settingsTitle:"Pengaturan",languageLabel:"Bahasa",languageAuto:"Ikuti bahasa Windows",advancedSettings:"Pengaturan lanjutan",advancedWarning:"Pengaturan ini dapat memengaruhi data yang dikumpulkan. Ubah hanya jika Anda memahami dampaknya.",durationLabel:"Durasi pengukuran (detik)",intervalLabel:"Interval telemetri (ms)",excludeDropped:"Kecualikan frame yang terlewat",noProcessProbing:"Nonaktifkan pemeriksaan thread/proses tambahan",presentmonPath:"Lokasi PresentMon",browse:"Pilih file",presentmonPrivacy:"Lokasi ini disimpan di perangkat Anda dan tidak dikirim melalui jaringan.",saveSettings:"Simpan pengaturan",settingsSaved:"Pengaturan disimpan di perangkat ini.",invalidAdvanced:"Periksa durasi (1–3600 detik) dan interval (10–5000 ms).",
    stopConfirmTitle:"Hentikan pengukuran?",stopConfirmBody:"Hasil mungkin belum cukup untuk menarik kesimpulan. Data yang tersedia tetap akan dianalisis sebagai hasil yang dihentikan lebih awal.",continueMeasure:"Lanjut mengukur",stopNow:"Hentikan sekarang",closeConfirmTitle:"Pengukuran masih berlangsung",closeConfirmBody:"Jika Anda menutup RWP, pengukuran akan dihentikan dan RWP akan mencoba menyimpan data yang tersedia.",stayOpen:"Tetap di RWP",closeAndStop:"Tutup dan hentikan pengukuran",permissionRepairTitle:"Izin Windows diperlukan",componentRepairTitle:"Komponen pengukuran diperlukan",understood:"Mengerti",
    permissionRepairBody:"<p>Windows belum mengizinkan RWP memulai pengukuran.</p><p>Di PC pribadi, tutup RWP lalu jalankan dengan memilih <strong>Run as administrator</strong>. RWP tidak menaikkan izin secara otomatis.</p><p>Alternatif: tambahkan akun Windows ke grup <strong>Performance Log Users</strong>.</p>",componentRepairBody:"<p>RWP memerlukan PresentMon versi yang didukung.</p><p>Dapatkan dari sumber resmi PresentMon lalu pilih file di sini. RWP tidak mengunduh atau memasang komponen ini otomatis.</p>",
    resultCompleted:"Selesai",resultInterrupted:"Dihentikan lebih awal",resultFailed:"Pengukuran gagal",verdictCpuTitle:"Ada indikasi hambatan prosesor",verdictCpuReason:"Bukti pengukuran mengindikasikan prosesor mungkin membatasi performa.",verdictCpuAdvice:"Coba kurangi pengaturan game yang membebani prosesor, lalu ukur ulang.",
    verdictGpuTitle:"Ada indikasi hambatan grafis",verdictGpuReason:"Bukti pengukuran mengindikasikan kemampuan grafis mungkin membatasi performa.",verdictGpuAdvice:"Coba kurangi kualitas grafis, lalu ukur ulang.",verdictPacingTitle:"Waktu antar-frame terindikasi kurang konsisten",verdictPacingReason:"Bukti pengukuran menunjukkan pola waktu antar-frame yang perlu diperhatikan.",verdictPacingAdvice:"Periksa detail pengukuran dan ulangi dengan kondisi yang serupa.",
    verdictThermalTitle:"Ada indikasi masalah terkait suhu",verdictThermalReason:"Bukti pengukuran mengindikasikan kemungkinan pengaruh kondisi termal.",verdictThermalAdvice:"Periksa pendinginan dan ventilasi PC.",verdictMemoryTitle:"Ada indikasi tekanan pada memori",verdictMemoryReason:"Bukti pengukuran mengindikasikan tekanan pada penggunaan memori.",verdictMemoryAdvice:"Tutup aplikasi yang tidak diperlukan, lalu ukur ulang.",
    verdictVramTitle:"Ada indikasi tekanan pada memori grafis",verdictVramReason:"Bukti pengukuran mengindikasikan tekanan pada memori grafis.",verdictVramAdvice:"Coba turunkan resolusi atau kualitas tekstur, lalu ukur ulang.",verdictBackgroundTitle:"Aktivitas latar mungkin memengaruhi hasil",verdictBackgroundReason:"Bukti pengukuran mengindikasikan kemungkinan gangguan dari aktivitas latar.",verdictBackgroundAdvice:"Tutup aplikasi latar yang tidak diperlukan dan ukur ulang.",
    verdictStableTitle:"Belum ada masalah yang terdeteksi",verdictStableReason:"Hasil memenuhi kriteria stabilitas yang ada; ini bukan bukti bahwa game pasti lancar.",verdictStableAdvice:"Bandingkan dengan pengukuran lain dalam kondisi yang serupa.",verdictStableFindingsTitle:"Hasil stabilitas tersedia, tetapi ada temuan lain",verdictStableFindingsReason:"Pengukuran memenuhi kriteria stabilitas, tetapi ada temuan lain yang perlu diperhatikan.",
    verdictUnknownTitle:"Belum bisa disimpulkan",verdictUnknownReason:"Data atau bukti yang tersedia belum mendukung kesimpulan yang dapat diandalkan.",verdictUnknownAdvice:"Coba ukur lagi hingga selesai.",verdictInterruptedAdvice:"Hasil dihentikan lebih awal. Ukur lagi hingga selesai untuk mendapatkan hasil yang lebih lengkap.",verdictStreamingReason:"Tidak ada aturan analisis untuk kategori ini; gunakan hasil belum bisa disimpulkan.",verdictStreamingAdvice:"Tidak ada saran khusus untuk kategori ini.",
    errorPermissionTitle:"Izin Windows diperlukan",errorPermissionBody:"RWP tidak dapat memulai pengukuran karena izin Windows tidak mencukupi.",errorComponentTitle:"Komponen pengukuran tidak tersedia",errorComponentBody:"RWP tidak dapat menemukan atau menggunakan komponen yang diperlukan untuk merekam performa.",errorPresentMonTitle:"Komponen pengukuran tidak dapat digunakan",errorPresentMonBody:"Periksa komponen PresentMon di Pengaturan lanjutan, lalu coba lagi.",errorIntervalTitle:"Pengaturan pengukuran tidak valid",errorIntervalBody:"Pilih interval telemetri yang didukung di Pengaturan lanjutan.",
    errorDurationTitle:"Durasi pengukuran tidak valid",errorDurationBody:"Pilih durasi pengukuran yang didukung di Pengaturan lanjutan.",errorTargetTitle:"Game tidak ditemukan",errorTargetBody:"Buka game dan pilih aplikasi yang sedang berjalan, lalu coba lagi.",errorSessionTitle:"Pengukuran masih berlangsung",errorSessionBody:"Tunggu pengukuran saat ini selesai sebelum memulai yang baru.",errorWorkerTitle:"Pengukuran tidak dapat dimulai",errorWorkerBody:"Coba mulai lagi. Jika masalah berulang, salin detail teknis untuk membantu menemukan penyebabnya.",errorVersionTitle:"Versi komponen tidak didukung",errorVersionBody:"Pilih versi PresentMon yang didukung.",errorUnknownTitle:"Terjadi masalah yang tidak terduga",errorUnknownBody:"Pengukuran tidak dapat diselesaikan. Detail teknis dapat membantu menemukan penyebabnya.",
    copyTechnical:"Salin detail teknis",technicalDetails:"Detail teknis",unknownCopy:"Detail teknis tersalin.",copyFailed:"Detail teknis tidak dapat disalin.",retry:"Coba lagi",failureActionPermission:"Lihat cara memperbaiki",failureActionComponent:"Pilih komponen",failureActionTarget:"Pilih game lagi",permissionRepairHint:"Ikuti petunjuk perbaikan izin Windows.",componentRepairHint:"Pilih komponen pengukuran yang didukung.",technicalHint:"Buka Detail teknis jika masalah berulang.",
    noData:"Tidak tersedia",expertMetrics:"Metrik",expertDiagnostics:"Bukti diagnosis",expertScore:"Skor bukti",expertRule:"ID aturan",expertAverageFps:"FPS rata-rata",expertP99:"Waktu frame P99",expertOneLow:"1% low",expertStutters:"Jumlah peristiwa stutter",expertSupporting:"Bukti pendukung",expertContradicting:"Bukti yang bertentangan",
    historyFrames:"{count} frame",sessionLabel:"Sesi {id}",verified:"Data terverifikasi",incomplete:"Data tidak utuh",stoppedEarly:"Dihentikan lebih awal",openResult:"Buka hasil",setBefore:"Jadikan Sebelum",setAfter:"Jadikan Sesudah",beforeSelected:"Dipilih sebagai Sebelum",afterSelected:"Dipilih sebagai Sesudah",instanceLabel:"instans {number}"
  },
  en: {
    navHome:"Home",navHistory:"History",navSettings:"Settings",homeEyebrow:"GAME PERFORMANCE",homeTitle:"Measure your game's performance",homeSubtitle:"See your game's performance measurement results on your PC.",
    gameLabel:"Choose a game",gameHint:"Choose the application you want to measure.",refresh:"Refresh",runningApps:"Running applications",chooseApp:"Choose an application",gameEmpty:"Open your game first, then select Refresh.",
    readinessTitle:"Measurement readiness",permissionLabel:"Measurement permissions",permissionReady:"Ready",permissionNeeded:"Needs attention",componentLabel:"Measurement component",componentReady:"Ready",componentNeeded:"Needs setup",fixIt:"View how to fix it",setupComponent:"Set up component",readinessNote:"RWP will check again before measuring.",
    quickOptions:"Measurement options",prepLabel:"Preparation time",prep5:"5 seconds",prep10:"10 seconds",prep15:"15 seconds",notifyStable:"Notify me if the result is stable",notifyStableHint:"The notification only appears after the measurement finishes.",startMeasure:"Start measuring",ready:"Ready to measure",checking:"Checking readiness…",
    noProcess:"No applications found. Open your game, then select Refresh.",refreshFailed:"The application list could not be refreshed.",preflightFailed:"Measurement readiness could not be checked. Try again.",countdownEyebrow:"GET READY",countdownTitle:"Getting ready to measure",countdownBody:"Switch to your game.",countdownHint:"Measurement starts after the countdown.",countdownAnnouncement:"Switch to your game. Measurement starts in {seconds} seconds.",cancel:"Cancel",
    measureEyebrow:"MEASUREMENT IN PROGRESS",recordingTitle:"Recording",recordingBody:"Recording. Play as usual; your results will appear when recording is complete.",recordingFooter:"Your results will appear after measurement finishes.",stopMeasure:"Stop measurement",analyzingEyebrow:"FINAL STEP",analyzingTitle:"Analyzing…",analyzingBody:"Preparing your measurement results.",analyzingHint:"RWP is finalizing results. Stop is disabled.",
    resultEyebrow:"MEASUREMENT RESULTS",resultHeading:"Measurement results",reasonLabel:"Reason",adviceLabel:"What to try next",alsoDetected:"Also detected",expertDetails:"Expert details",measureAgain:"Measure again",compare:"Compare",compareDisabled:"There is no other measurement from this game available to compare yet.",
    stabilityUnknown:"Stability not yet known",stabilityUnknownBody:"There is not enough evidence to determine stability.",stabilityStable:"Stable",stabilityStableBody:"The measurement meets the existing stability criteria. This does not guarantee the game will always run smoothly.",stabilityUnstable:"Unstable",stabilityUnstableBody:"The available measurement contains stutter events.",observations:"Observations: {count}",stutters:"Stutter events: {count}",
    historyEyebrow:"SAVED LOCALLY",historyTitle:"Measurement history",historySubtitle:"Select a measurement to view results or compare measurements.",savedMeasurements:"Saved measurements",historyEmptyTitle:"No measurement history yet",historyEmptyBody:"Saved measurements will appear here.",measureGame:"Measure a game",
    compareTitle:"Before / After",compareHint:"Choose a session from the list using Set as Before or Set as After.",beforeLabel:"Before",afterLabel:"After",notSelected:"Not selected",scenarioQuestion:"Were the conditions for both measurements sufficiently similar?",scenarioYes:"Yes, the conditions were comparable",scenarioUnsure:"Not sure",scenarioNo:"No, the conditions were different",compareAction:"Compare results",compareSelectHint:"Select two verified measurements from the same application.",scenarioRequired:"Choose an answer about measurement conditions.",
    compareComparable:"Results are comparable",compareComparableBody:"Both measurements meet the available comparability criteria.",compareLimited:"Limited comparison",compareLimitedBody:"The two measurements contain different amounts of data. Compare the results with care.",compareInconclusive:"Inconclusive",compareInconclusiveBody:"The two measurements cannot be confirmed as comparable in their conditions.",compareAdvice:"Repeat measurements under similar conditions to make comparison easier.",sameApplication:"Select two measurements from the same application.",verifiedRequired:"Session data is not verified and cannot be selected.",compareFailed:"The comparison could not be completed.",
    settingsEyebrow:"LOCAL PREFERENCES",settingsTitle:"Settings",languageLabel:"Language",languageAuto:"Follow Windows language",advancedSettings:"Advanced settings",advancedWarning:"These settings can affect the data collected. Change them only if you understand their impact.",durationLabel:"Measurement duration (seconds)",intervalLabel:"Telemetry interval (ms)",excludeDropped:"Exclude dropped presents",noProcessProbing:"Disable additional process/thread probing",presentmonPath:"PresentMon location",browse:"Choose file",presentmonPrivacy:"This location stays on your device and is not sent over the network.",saveSettings:"Save settings",settingsSaved:"Settings saved on this device.",invalidAdvanced:"Check duration (1–3600 seconds) and interval (10–5000 ms).",
    stopConfirmTitle:"Stop the measurement?",stopConfirmBody:"There may not be enough data to draw a conclusion. Available data will still be analyzed as an early-stopped result.",continueMeasure:"Continue measuring",stopNow:"Stop now",closeConfirmTitle:"Measurement is still in progress",closeConfirmBody:"If you close RWP, measurement will stop and RWP will try to save available data.",stayOpen:"Keep RWP open",closeAndStop:"Close and stop measurement",permissionRepairTitle:"Windows permission required",componentRepairTitle:"Measurement component required",understood:"Got it",
    permissionRepairBody:"<p>Windows has not allowed RWP to start measurement.</p><p>On your own PC, close RWP and open it using <strong>Run as administrator</strong>. RWP never elevates silently.</p><p>Alternatively, ask an administrator to add your account to the <strong>Performance Log Users</strong> group.</p>",componentRepairBody:"<p>RWP requires a supported copy of PresentMon.</p><p>Get it from the official PresentMon source and choose the file here. RWP does not download or install the component automatically.</p>",
    resultCompleted:"Completed",resultInterrupted:"Stopped early",resultFailed:"Measurement failed",verdictCpuTitle:"Possible CPU bottleneck",verdictCpuReason:"Measurement evidence suggests the CPU may be limiting performance.",verdictCpuAdvice:"Try lowering CPU-intensive game settings, then measure again.",
    verdictGpuTitle:"Possible graphics bottleneck",verdictGpuReason:"Measurement evidence suggests graphics performance may be limiting the result.",verdictGpuAdvice:"Try lowering graphics quality, then measure again.",verdictPacingTitle:"Frame pacing may be inconsistent",verdictPacingReason:"Measurement evidence indicates a frame-timing pattern worth investigating.",verdictPacingAdvice:"Review the measurement details and repeat under similar conditions.",
    verdictThermalTitle:"Possible thermal issue",verdictThermalReason:"Measurement evidence suggests a possible thermal influence.",verdictThermalAdvice:"Check your PC's cooling and ventilation.",verdictMemoryTitle:"Possible memory pressure",verdictMemoryReason:"Measurement evidence suggests memory pressure.",verdictMemoryAdvice:"Close unnecessary applications, then measure again.",
    verdictVramTitle:"Possible graphics memory pressure",verdictVramReason:"Measurement evidence suggests graphics memory pressure.",verdictVramAdvice:"Try lowering resolution or texture quality, then measure again.",verdictBackgroundTitle:"Background activity may affect results",verdictBackgroundReason:"Measurement evidence suggests possible background interference.",verdictBackgroundAdvice:"Close unnecessary background applications and measure again.",
    verdictStableTitle:"No issues detected",verdictStableReason:"The result meets the existing stability criteria; this does not prove the game is always smooth.",verdictStableAdvice:"Compare with another measurement taken under similar conditions.",verdictStableFindingsTitle:"Stability result available, but other findings remain",verdictStableFindingsReason:"The measurement meets stability criteria, but other findings still need attention.",
    verdictUnknownTitle:"Inconclusive",verdictUnknownReason:"The available data or evidence does not support a reliable conclusion.",verdictUnknownAdvice:"Try measuring again and let it finish.",verdictInterruptedAdvice:"This result was stopped early. Measure again and let it finish for a more complete result.",verdictStreamingReason:"No analysis rule exists for this category; use the inconclusive fallback.",verdictStreamingAdvice:"No category-specific advice is available.",
    errorPermissionTitle:"Windows permission required",errorPermissionBody:"RWP could not start measurement because Windows permissions are insufficient.",errorComponentTitle:"Measurement component unavailable",errorComponentBody:"RWP cannot find or use the component required to record performance.",errorPresentMonTitle:"Measurement component cannot be used",errorPresentMonBody:"Check the PresentMon component in Advanced settings, then try again.",errorIntervalTitle:"Measurement setting is invalid",errorIntervalBody:"Choose a supported telemetry interval in Advanced settings.",
    errorDurationTitle:"Measurement duration is invalid",errorDurationBody:"Choose a supported measurement duration in Advanced settings.",errorTargetTitle:"Game not found",errorTargetBody:"Open the game and select the running application, then try again.",errorSessionTitle:"Measurement is still in progress",errorSessionBody:"Wait for the current measurement to finish before starting another.",errorWorkerTitle:"Measurement could not start",errorWorkerBody:"Try starting again. If the problem continues, copy the technical details to help identify the cause.",errorVersionTitle:"Component version not supported",errorVersionBody:"Choose a supported PresentMon version.",errorUnknownTitle:"An unexpected problem occurred",errorUnknownBody:"The measurement could not be completed. Technical details may help identify the cause.",
    copyTechnical:"Copy technical details",technicalDetails:"Technical details",unknownCopy:"Technical details copied.",copyFailed:"Technical details could not be copied.",retry:"Try again",failureActionPermission:"View how to fix it",failureActionComponent:"Choose component",failureActionTarget:"Choose game again",permissionRepairHint:"Follow the Windows permission repair instructions.",componentRepairHint:"Choose a supported measurement component.",technicalHint:"Open Technical details if the problem continues.",
    noData:"Not available",expertMetrics:"Metrics",expertDiagnostics:"Diagnostic evidence",expertScore:"Evidence score",expertRule:"Rule ID",expertAverageFps:"Average FPS",expertP99:"P99 frame time",expertOneLow:"1% low",expertStutters:"Stutter event count",expertSupporting:"Supporting evidence",expertContradicting:"Contradicting evidence",
    historyFrames:"{count} frames",sessionLabel:"Session {id}",verified:"Data verified",incomplete:"Data incomplete",stoppedEarly:"Stopped early",openResult:"Open result",setBefore:"Set as Before",setAfter:"Set as After",beforeSelected:"Selected as Before",afterSelected:"Selected as After",instanceLabel:"instance {number}"
  }
};

function storageGet(key, fallback) {
  try { const value = localStorage.getItem(key); return value === null ? fallback : value; } catch (_) { return fallback; }
}
function storageSet(key, value) {
  try { localStorage.setItem(key, String(value)); } catch (_) { /* local preferences are best-effort */ }
}
function detectLocale(preference) {
  if (preference === "id" || preference === "en") return preference;
  return String(navigator.language || "id").toLowerCase().startsWith("en") ? "en" : "id";
}
const state = {
  languagePreference: storageGet("rwp.language", "auto"),
  locale: "id",
  view: "home",
  flow: "idle",
  presentmonPath: storageGet("rwp.presentmonPath", ""),
  processId: "",
  processes: [],
  preflight: null,
  snapshot: null,
  history: [],
  beforeId: "",
  afterId: "",
  prepSeconds: Number(storageGet("rwp.prepSeconds", "5")) || 5,
  durationSeconds: Number(storageGet("rwp.durationSeconds", "60")) || 60,
  intervalMs: Number(storageGet("rwp.intervalMs", "50")) || 50,
  notifyStable: storageGet("rwp.notifyStable", "true") !== "false",
  excludeDropped: storageGet("rwp.excludeDropped", "false") === "true",
  noProcessProbing: storageGet("rwp.noProcessProbing", "false") === "true",
  stopRequested: false,
  stopCallSent: false,
  startRequested: false,
  closeAfterTerminal: false,
  lastError: "",
  lastErrorKind: "unknown",
  pollTimer: null,
  pollInFlight: false,
  countdownTimer: null,
  countdownDeadline: 0,
  countdownLast: -1,
  toastTimer: null
};
state.locale = detectLocale(state.languagePreference);

function $(id) { return document.getElementById(id); }
const el = {
  views: Array.from(document.querySelectorAll(".view")),
  nav: Array.from(document.querySelectorAll("[data-view]")),
  processSelect:$("process-select"), gameEmpty:$("game-empty"), processStatus:$("process-status"),
  permissionStatus:$("permission-status"), permissionIcon:$("permission-icon"), permissionFix:$("permission-fix"),
  componentStatus:$("component-status"), componentIcon:$("component-icon"), componentSetup:$("component-setup"),
  homeMessage:$("home-message"), start:$("start-session"), countdownStage:$("countdown-stage"), countdownNumber:$("countdown-number"),
  countdownAnnouncement:$("countdown-announcement"), recordingStage:$("recording-stage"), analyzingStage:$("analyzing-stage"),
  elapsed:$("elapsed-duration"), progress:$("recording-progress"), progressFill:$("progress-fill"), progressCaption:$("progress-caption"),
  stopMeasurement:$("stop-measurement"), resultMeta:$("result-meta"), resultSuccess:$("result-success-panel"), resultFailure:$("result-failure-panel"),
  resultOutcome:$("result-outcome"), verdictTitle:$("verdict-title"), verdictReason:$("verdict-reason"), verdictAdvice:$("verdict-advice"),
  stabilityHeading:$("stability-heading"), stabilityDot:$("stability-dot"), stabilityDescription:$("stability-description"),
  stabilityObservations:$("stability-observations"), stabilityStutters:$("stability-stutters"), otherSection:$("other-diagnoses-section"),
  otherList:$("other-diagnoses-list"), expertContent:$("expert-content"), resultCompare:$("compare-from-result"),
  resultCompareExplanation:$("result-compare-explanation"), failureTitle:$("failure-title"), failureBody:$("failure-body"),
  failureAdvice:$("failure-advice"), technicalError:$("technical-error"), retryAction:$("retry-action"),
  historyList:$("history-list"), historyMessage:$("history-message"), beforeLabel:$("before-label"), afterLabel:$("after-label"),
  compareGuidance:$("compare-guidance"), runCompare:$("run-compare"), compareResult:$("compare-result"),
  compareStatusTitle:$("compare-status-title"), compareStatusBody:$("compare-status-body"), compareMetrics:$("compare-metrics"),
  languageSetting:$("language-setting"), duration:$("duration-seconds"), interval:$("sample-interval-ms"),
  excludeDropped:$("exclude-dropped"), noProcessProbing:$("no-process-probing"), presentmonPath:$("presentmon-path"),
  settingsMessage:$("settings-message"), toast:$("toast"), stopDialog:$("stop-dialog"), closeDialog:$("close-dialog"),
  repairDialog:$("repair-dialog"), repairTitle:$("repair-title"), repairBody:$("repair-body")
};

function t(key, values) {
  const dictionary = STRINGS[state.locale] || STRINGS.id;
  let value = dictionary[key] || STRINGS.en[key] || key;
  if (values) value = value.replace(/\{(\w+)\}/g, function(match, name) {
    return Object.prototype.hasOwnProperty.call(values, name) ? String(values[name]) : match;
  });
  return value;
}
function applyLocale() {
  const currentView = state.view;
  document.documentElement.lang = state.locale;
  document.querySelectorAll("[data-i18n]").forEach(function(node) {
    node.textContent = t(node.getAttribute("data-i18n"));
  });
  if (el.languageSetting) el.languageSetting.value = state.languagePreference;
  syncPreferenceControls();
  if (state.preflight) renderPreflight();
  renderHistory();
  if (state.snapshot && state.snapshot.status === "completed") renderCompleted(state.snapshot);
  if (state.snapshot && state.snapshot.status === "failed") renderFailure(state.snapshot.message || state.lastError);
  if (state.view !== currentView) setView(currentView);
}
function escapeHtml(value) {
  return String(value === undefined || value === null ? "" : value).replaceAll("&","&amp;").replaceAll("<","&lt;").replaceAll(">","&gt;").replaceAll('"',"&quot;").replaceAll("'","&#039;");
}
function cleanApp(value) { return String(value || "").trim().replace(/\.exe$/i, "").trim(); }
function normApp(value) { return cleanApp(value).toLocaleLowerCase("en-US"); }
function showToast(message) {
  el.toast.textContent = message;
  el.toast.hidden = false;
  if (state.toastTimer) clearTimeout(state.toastTimer);
  state.toastTimer = setTimeout(function() { el.toast.hidden = true; }, 3500);
}
function setView(name) {
  state.view = name;
  el.views.forEach(function(view) {
    const active = view.id === "view-" + name;
    view.hidden = !active;
    view.classList.toggle("active", active);
  });
  const interactionLocked = ["countdown", "recording", "analyzing"].includes(state.flow);
  el.nav.forEach(function(button) {
    const active = button.getAttribute("data-view") === name;
    button.classList.toggle("active", active);
    if (active && button.classList.contains("nav-button")) button.setAttribute("aria-current","page");
    else button.removeAttribute("aria-current");
    if (button.tagName === "BUTTON") button.disabled = interactionLocked;
    else {
      button.setAttribute("aria-disabled", interactionLocked ? "true" : "false");
      button.tabIndex = interactionLocked ? -1 : 0;
    }
  });
  if (name === "home") { refreshPreflight(); }
  if (name === "history") { refreshHistory(); }
  if (name === "settings") { loadSettings(); }
}
function setReadiness(node, icon, ready, successCopy, missingCopy) {
  node.textContent = ready ? successCopy : missingCopy;
  node.classList.toggle("ready", Boolean(ready));
  node.classList.toggle("needs-action", !ready);
  icon.textContent = ready ? "✓" : "!";
  icon.classList.toggle("ready", Boolean(ready));
  icon.classList.toggle("needs-action", !ready);
}
function renderPreflight() {
  if (!state.preflight) {
    setReadiness(el.permissionStatus, el.permissionIcon, false, t("permissionReady"), t("checking"));
    setReadiness(el.componentStatus, el.componentIcon, false, t("componentReady"), t("checking"));
    el.start.disabled = true;
    return;
  }
  const permission = Boolean(state.preflight.permission_ready);
  const component = Boolean(state.preflight.component_ready);
  setReadiness(el.permissionStatus, el.permissionIcon, permission, t("permissionReady"), t("permissionNeeded"));
  setReadiness(el.componentStatus, el.componentIcon, component, t("componentReady"), t("componentNeeded"));
  el.permissionFix.hidden = permission;
  el.componentSetup.hidden = component;
  const processReady = Boolean(state.processId && state.preflight.process_ready);
  el.start.disabled = !(invoke && permission && component && processReady);
  if (el.start.disabled && !state.processId) el.homeMessage.textContent = state.processes.length ? t("chooseApp") : t("noProcess");
  else if (el.start.disabled && (!permission || !component)) el.homeMessage.textContent = t("readinessNote");
  else if (!el.start.disabled) el.homeMessage.textContent = t("ready");
}
async function refreshPreflight() {
  if (!invoke) {
    state.preflight = {permission_ready:false,component_ready:false,process_ready:false};
    renderPreflight();
    return;
  }
  try {
    state.preflight = await invoke("session_preflight", {
      presentmonExecutable:state.presentmonPath || null,
      processId:state.processId ? Number(state.processId) : null
    });
    renderPreflight();
  } catch (_) {
    state.preflight = {permission_ready:false,component_ready:false,process_ready:false};
    renderPreflight();
    el.homeMessage.textContent = t("preflightFailed");
  }
}
async function refreshProcesses() {
  if (!invoke) { el.processStatus.textContent = t("refreshFailed"); return; }
  const previous = state.processId;
  try {
    const rows = await invoke("session_processes");
    state.processes = Array.isArray(rows) ? rows.slice() : [];
    state.processes.sort(function(a,b) {
      const names = String(a.name || "").localeCompare(String(b.name || ""), "en");
      return names || Number(a.process_id) - Number(b.process_id);
    });
    const counts = {};
    state.processes.forEach(function(row) { const key=normApp(row.name); counts[key]=(counts[key]||0)+1; });
    const indices = {};
    let options = '<option value="">' + escapeHtml(t("chooseApp")) + '</option>';
    state.processes.forEach(function(row) {
      const name = cleanApp(row.name), key = normApp(name);
      indices[key] = (indices[key] || 0) + 1;
      const label = counts[key] > 1 ? name + " (" + t("instanceLabel",{number:indices[key]}) + ")" : name;
      options += '<option value="' + escapeHtml(row.process_id) + '">' + escapeHtml(label) + '</option>';
    });
    el.processSelect.innerHTML = options;
    state.processId = previous && state.processes.some(function(row) { return String(row.process_id) === String(previous); }) ? previous : "";
    el.processSelect.value = state.processId;
    el.gameEmpty.hidden = state.processes.length > 0;
    el.processStatus.textContent = state.processes.length ? "" : t("noProcess");
    await refreshPreflight();
  } catch (_) {
    state.processes = []; state.processId = ""; el.processSelect.value = "";
    el.gameEmpty.hidden = false; el.processStatus.textContent = t("refreshFailed");
    await refreshPreflight();
  }
}
async function choosePresentMon() {
  if (!invoke) return;
  try {
    const chosen = await invoke("session_pick_presentmon_file");
    if (!chosen) return;
    state.presentmonPath = String(chosen);
    storageSet("rwp.presentmonPath", state.presentmonPath);
    if (el.presentmonPath) el.presentmonPath.value = state.presentmonPath;
    await refreshPreflight();
  } catch (_) {
    showToast(t("componentNeeded"));
  }
}
function syncPreferenceControls() {
  document.querySelectorAll("[data-prep-choice]").forEach(function(input) { input.checked = Number(input.value) === state.prepSeconds; });
  document.querySelectorAll("[data-notify-stable]").forEach(function(input) { input.checked = state.notifyStable; });
}
function formatClock(seconds) {
  const total = Math.max(0, Math.floor(seconds));
  return String(Math.floor(total/60)).padStart(2,"0") + ":" + String(total%60).padStart(2,"0");
}
function stopCountdown() {
  if (state.countdownTimer) clearInterval(state.countdownTimer);
  state.countdownTimer = null; state.countdownDeadline = 0; state.countdownLast = -1;
}
function showMeasureStage(stage) {
  state.flow = stage; setView("measure");
  el.countdownStage.hidden = stage !== "countdown";
  el.recordingStage.hidden = stage !== "recording";
  el.analyzingStage.hidden = stage !== "analyzing";
}
async function beginCountdown() {
  if (state.startRequested || state.countdownTimer) return;
  await refreshPreflight();
  if (!state.preflight || !state.preflight.permission_ready) { openRepair("permission"); return; }
  if (!state.preflight.component_ready) { openRepair("component"); return; }
  if (!state.processId || !state.preflight.process_ready) {
    setView("home"); await refreshProcesses(); showToast(t("errorTargetBody")); return;
  }
  state.stopRequested = false; state.stopCallSent = false; state.startRequested = false;
  state.countdownDeadline = performance.now() + state.prepSeconds * 1000;
  showMeasureStage("countdown"); tickCountdown();
  state.countdownTimer = setInterval(tickCountdown, 100);
}
function tickCountdown() {
  if (!state.countdownDeadline || state.flow !== "countdown") return;
  const remainingMs = Math.max(0, state.countdownDeadline - performance.now());
  const seconds = Math.ceil(remainingMs/1000);
  el.countdownNumber.textContent = String(seconds);
  if (seconds !== state.countdownLast) {
    state.countdownLast = seconds;
    if (seconds > 0) el.countdownAnnouncement.textContent = t("countdownAnnouncement",{seconds:seconds});
  }
  if (remainingMs <= 0 && !state.startRequested) {
    stopCountdown(); state.startRequested = true; startSessionAfterCountdown();
  }
}
async function startSessionAfterCountdown() {
  await refreshPreflight();
  if (!state.preflight || !state.preflight.permission_ready) { state.startRequested=false; showFailure("permission",""); return; }
  if (!state.preflight.component_ready) { state.startRequested=false; showFailure("component",""); return; }
  if (!state.processId || !state.preflight.process_ready) { state.startRequested=false; showFailure("target",""); return; }
  try {
    const snapshot = await invoke("session_start_managed", { request:{
      presentmon_executable:state.presentmonPath,
      process_id:Number(state.processId),
      duration_seconds:state.durationSeconds,
      sample_interval_ms:state.intervalMs,
      exclude_dropped:state.excludeDropped,
      no_process_probing:state.noProcessProbing,
      disable_stable_notification:!state.notifyStable,
      notification_language:state.locale
    }});
    state.startRequested = false; state.snapshot = snapshot;
    if (snapshot && snapshot.status === "running") { showMeasureStage("recording"); startPolling(); await handleSnapshot(snapshot); }
    else await handleSnapshot(snapshot);
  } catch (error) {
    state.startRequested=false; stopCountdown(); await showFailureFromMessage(String(error));
  }
}
function startPolling() {
  if (state.pollTimer) clearInterval(state.pollTimer);
  state.pollTimer = setInterval(pollStatus, 500);
  pollStatus();
}
function clearPolling() { if (state.pollTimer) clearInterval(state.pollTimer); state.pollTimer = null; }
async function pollStatus() {
  if (!invoke || state.pollInFlight) return;
  state.pollInFlight = true;
  try { await handleSnapshot(await invoke("session_status")); }
  catch (_) { showToast(t("preflightFailed")); }
  finally { state.pollInFlight = false; }
}
function updateProgress(snapshot) {
  const started=Number(snapshot.started_unix_ms), requested=Number(snapshot.duration_seconds);
  if (!Number.isFinite(started) || !Number.isFinite(requested) || requested<=0) return;
  const elapsed=Math.max(0,(Date.now()-started)/1000), percent=Math.round(Math.max(0,Math.min(1,elapsed/requested))*100);
  el.elapsed.textContent=formatClock(elapsed)+" / "+formatClock(requested);
  el.progressFill.style.width=percent+"%";
  el.progress.setAttribute("aria-valuenow",String(percent));
  el.progress.setAttribute("aria-valuetext",percent+"%");
  el.progressCaption.textContent=percent+"%";
}
async function handleSnapshot(snapshot) {
  if (!snapshot || typeof snapshot.status !== "string") return;
  state.snapshot = snapshot;
  if (snapshot.status === "idle") {
    if (state.flow === "recording" || state.flow === "analyzing") {
      clearPolling(); state.flow="idle"; showFailure("unknown","Session ended without a result.");
    }
    return;
  }
  if (snapshot.status === "running") {
    if (state.flow === "countdown") return;
    const elapsed=Number(snapshot.started_unix_ms)>0 ? Math.max(0,(Date.now()-Number(snapshot.started_unix_ms))/1000) : 0;
    if (state.stopRequested || elapsed >= Number(snapshot.duration_seconds || state.durationSeconds)) showMeasureStage("analyzing");
    else { showMeasureStage("recording"); updateProgress(snapshot); }
    return;
  }
  clearPolling();
  if (snapshot.status === "failed") {
    state.flow="result"; await showFailureFromMessage(snapshot.message || state.lastError || "");
    if (state.closeAfterTerminal) await finalizeWindowClose();
    return;
  }
  if (snapshot.status === "completed") {
    state.flow="result"; renderCompleted(snapshot); await refreshHistory();
    updateResultCompareAvailability();
    if (state.closeAfterTerminal) await finalizeWindowClose();
  }
}
function diagnosisCopy(ruleId) {
  const map={
    "cpu_bound.v1":["verdictCpuTitle","verdictCpuReason","verdictCpuAdvice"],
    "gpu_bound.v2":["verdictGpuTitle","verdictGpuReason","verdictGpuAdvice"],
    "frame_pacing.v2":["verdictPacingTitle","verdictPacingReason","verdictPacingAdvice"],
    "thermal.v2":["verdictThermalTitle","verdictThermalReason","verdictThermalAdvice"],
    "memory_pressure.v1":["verdictMemoryTitle","verdictMemoryReason","verdictMemoryAdvice"],
    "vram_pressure.v1":["verdictVramTitle","verdictVramReason","verdictVramAdvice"],
    "background_interference.v3":["verdictBackgroundTitle","verdictBackgroundReason","verdictBackgroundAdvice"],
    "stability.v1":["verdictStableTitle","verdictStableReason","verdictStableAdvice"],
    "unknown":["verdictUnknownTitle","verdictUnknownReason","verdictUnknownAdvice"]
  };
  return (map[ruleId] || map.unknown).map(function(key){return t(key);});
}
function isStreamingDiagnostic(item) {
  return String(item.rule_id || "").startsWith("streaming_overhead") || String(item.category || "").toLowerCase().includes("streaming");
}
function renderCompleted(snapshot) {
  el.resultSuccess.hidden=false; el.resultFailure.hidden=true;
  const app=cleanApp(snapshot.application);
  const rawFrameCount = snapshot.frame_count;
  const frameCount = Number.isSafeInteger(rawFrameCount) && rawFrameCount >= 0
    ? rawFrameCount
    : null;
  el.resultMeta.textContent = app + " · " + (
    frameCount === null ? t("noData") : t("historyFrames", {count:frameCount})
  );
  el.resultOutcome.textContent=snapshot.interrupted ? t("resultInterrupted") : t("resultCompleted");
  const dashboard=snapshot.dashboard || {};
  const allDiagnostics = Array.isArray(dashboard.diagnostics) ? dashboard.diagnostics : [];
  const diagnostics = allDiagnostics.filter(function(item) {
    return !isStreamingDiagnostic(item);
  });
  const rawStutters = dashboard.stutters;
  const stutters = Array.isArray(rawStutters) ? rawStutters : null;
  const primary=diagnostics.length ? diagnostics[0] : {rule_id:"unknown",category:"Unknown"};
  const stable=diagnostics.some(function(item){return item.rule_id==="stability.v1";});
  const copy=diagnosisCopy(primary.rule_id);
  if (stable && diagnostics.length>1) {
    el.verdictTitle.textContent=t("verdictStableFindingsTitle");
    el.verdictReason.textContent=t("verdictStableFindingsReason");
    const other=diagnostics.find(function(item){return item.rule_id!=="stability.v1";});
    el.verdictAdvice.textContent=other ? diagnosisCopy(other.rule_id)[2] : t("verdictStableAdvice");
  } else {
    el.verdictTitle.textContent=copy[0]; el.verdictReason.textContent=copy[1]; el.verdictAdvice.textContent=copy[2];
  }
  if (snapshot.interrupted) el.verdictAdvice.textContent=t("verdictInterruptedAdvice");
  const others = stable && diagnostics.length > 1
    ? diagnostics.filter(function(item) { return item.rule_id !== "stability.v1"; })
    : diagnostics.slice(1);
  el.otherList.replaceChildren();
  el.otherSection.hidden = others.length === 0;
  others.forEach(function(item) {
    const li = document.createElement("li");
    li.textContent = diagnosisCopy(item.rule_id)[0];
    el.otherList.appendChild(li);
  });
  const status = stable ? "stable" : "unknown";
  el.stabilityHeading.textContent = stable ? t("stabilityStable") : t("stabilityUnknown");
  el.stabilityDescription.textContent = stable ? t("stabilityStableBody") : t("stabilityUnknownBody");
  el.stabilityDot.className = "status-dot " + status;
  el.stabilityObservations.textContent = frameCount === null
    ? t("noData")
    : t("observations", {count:frameCount});
  el.stabilityStutters.textContent = stutters === null
    ? t("noData")
    : t("stutters", {count:stutters.length});
  renderExpertDetails(snapshot, allDiagnostics, stutters || []);
  setView("result"); state.lastError=""; refreshHistory().then(updateResultCompareAvailability);
}
function renderExpertDetails(snapshot,diagnostics,stutters) {
  const dashboard=snapshot.dashboard || {}, metrics=Array.isArray(dashboard.metrics) ? dashboard.metrics : [];
  const labels={average_fps:t("expertAverageFps"),p99_frame_time:t("expertP99"),one_percent_low:t("expertOneLow"),stutter_events:t("expertStutters")};
  const container=document.createElement("div");
  const metricHeading=document.createElement("h3"); metricHeading.textContent=t("expertMetrics"); container.appendChild(metricHeading);
  const grid=document.createElement("div"); grid.className="expert-grid";
  metrics.forEach(function(metric){
    const card=document.createElement("div");card.className="expert-metric";
    const label=document.createElement("span");label.textContent=labels[metric.id] || metric.title || t("noData");
    const value=document.createElement("strong");value.textContent=metric.value === undefined || metric.value === null ? t("noData") : String(metric.value);
    card.append(label,value);grid.appendChild(card);
  });
  container.appendChild(grid);
  const diagHeading=document.createElement("h3");diagHeading.textContent=t("expertDiagnostics");container.appendChild(diagHeading);
  diagnostics.forEach(function(item){
    const article=document.createElement("article");article.className="expert-evidence";
    const heading=document.createElement("h4");heading.textContent=String(item.category || t("verdictUnknownTitle"));article.appendChild(heading);
    const score=document.createElement("p");score.className="expert-score";score.textContent=t("expertRule")+": "+String(item.rule_id||"unknown")+" · "+t("expertScore")+": "+(item.evidence_score===undefined?t("noData"):String(item.evidence_score))+"/100";article.appendChild(score);
    [["supporting_evidence","expertSupporting"],["contradicting_evidence","expertContradicting"]].forEach(function(pair){
      const entries=Array.isArray(item[pair[0]]) ? item[pair[0]] : [];
      if(!entries.length)return;
      const title=document.createElement("h4");title.textContent=t(pair[1]);article.appendChild(title);
      const list=document.createElement("ul");
      entries.forEach(function(raw){const li=document.createElement("li");li.textContent=String(raw);list.appendChild(li);});
      article.appendChild(list);
    });
    container.appendChild(article);
  });
  if(stutters.length){const count=document.createElement("p");count.textContent=t("stutters",{count:stutters.length});container.appendChild(count);}
  el.expertContent.replaceChildren(container);
}
function renderFailure(kind,message) {
  const titleKey={permission:"errorPermissionTitle",component:"errorComponentTitle",presentmon_name:"errorPresentMonTitle",interval:"errorIntervalTitle",duration:"errorDurationTitle",target:"errorTargetTitle",session_active:"errorSessionTitle",worker:"errorWorkerTitle",version:"errorVersionTitle",unknown:"errorUnknownTitle"};
  const bodyKey={permission:"errorPermissionBody",component:"errorComponentBody",presentmon_name:"errorPresentMonBody",interval:"errorIntervalBody",duration:"errorDurationBody",target:"errorTargetBody",session_active:"errorSessionBody",worker:"errorWorkerBody",version:"errorVersionBody",unknown:"errorUnknownBody"};
  state.lastErrorKind=kind || "unknown";state.lastError=String(message||"");
  el.resultSuccess.hidden=true;el.resultFailure.hidden=false;
  el.resultOutcome.textContent=t("resultFailed");
  el.failureTitle.textContent=t(titleKey[state.lastErrorKind]||titleKey.unknown);
  el.failureBody.textContent=t(bodyKey[state.lastErrorKind]||bodyKey.unknown);
  el.failureAdvice.textContent=state.lastErrorKind==="permission"?t("permissionRepairHint"):(["component","presentmon_name","version"].includes(state.lastErrorKind)?t("componentRepairHint"):(state.lastErrorKind==="target"?t("gameEmpty"):t("technicalHint")));
  el.retryAction.textContent = t("retry");
  el.technicalError.textContent=state.lastError || t("noData");
  state.flow="result";setView("result");
}
function showFailure(kind,message){renderFailure(kind,message);}
async function showFailureFromMessage(message) {
  state.lastError=String(message||"");
  let kind="unknown";
  if(invoke){try{kind=await invoke("session_error_kind",{message:state.lastError});}catch(_){kind="unknown";}}
  renderFailure(kind,state.lastError);
  if(state.closeAfterTerminal) await finalizeWindowClose();
}
function updateResultCompareAvailability() {
  if(!state.snapshot || state.snapshot.status!=="completed")return;
  const current=normApp(state.snapshot.application);
  const candidates=state.history.filter(function(item){return item.verified && normApp(item.application)===current;});
  const enabled=candidates.length>=2;
  el.resultCompare.disabled=!enabled;el.resultCompareExplanation.hidden=enabled;
}
async function refreshHistory() {
  if(!invoke){el.historyMessage.textContent=t("refreshFailed");return;}
  try{
    const entries=await invoke("session_history");
    state.history=Array.isArray(entries)?entries.slice():[];
    state.history.sort(function(a,b){return String(a.session_id||"").localeCompare(String(b.session_id||""),"en");});
    renderHistory();updateCompareButton();updateResultCompareAvailability();
  }catch(_){el.historyMessage.textContent=t("refreshFailed");}
}
function shortSessionId(raw) {
  const value = String(raw || "");
  if (value.length <= 12) return value;
  let prefixLength = 12;
  while (prefixLength < value.length) {
    const prefix = value.slice(0, prefixLength);
    const collisions = state.history.filter(function(entry) {
      return String(entry.session_id || "").startsWith(prefix);
    }).length;
    if (collisions <= 1) break;
    prefixLength = Math.min(value.length, prefixLength + 4);
  }
  return value.slice(0, prefixLength) + (prefixLength < value.length ? "…" : "");
}
function historyLabel(entry){return cleanApp(entry.application)+" · "+t("sessionLabel",{id:shortSessionId(entry.session_id)});}
function renderHistory() {
  if(!el.historyList)return;
  if(!state.history.length){
    el.historyList.innerHTML='<div class="empty-panel"><h3>'+escapeHtml(t("historyEmptyTitle"))+'</h3><p>'+escapeHtml(t("historyEmptyBody"))+'</p><button class="button secondary" type="button" data-view="home">'+escapeHtml(t("measureGame"))+'</button></div>';
  }else{
    el.historyList.innerHTML=state.history.map(function(entry){
      const valid=Boolean(entry.verified), before=state.beforeId===entry.session_id, after=state.afterId===entry.session_id;
      const status=entry.interrupted?t("stoppedEarly"):t("resultCompleted");
      return '<article class="history-row"><div><h3>'+escapeHtml(historyLabel(entry))+'</h3><p>'+escapeHtml(t("historyFrames",{count:entry.frame_count}))+' · '+escapeHtml(status)+'</p><div class="history-badges"><span class="history-badge '+(valid?"verified":"incomplete")+'">'+escapeHtml(valid?t("verified"):t("incomplete"))+'</span></div></div><div class="history-actions"><button class="button secondary" type="button" data-history-action="load" data-session-id="'+escapeHtml(entry.session_id)+'">'+escapeHtml(t("openResult"))+'</button><button class="button secondary" type="button" data-history-action="before" data-session-id="'+escapeHtml(entry.session_id)+'"'+(!valid?' disabled title="'+escapeHtml(t("verifiedRequired"))+'"':'')+' aria-pressed="'+before+'">'+escapeHtml(before?t("beforeSelected"):t("setBefore"))+'</button><button class="button secondary" type="button" data-history-action="after" data-session-id="'+escapeHtml(entry.session_id)+'"'+(!valid?' disabled title="'+escapeHtml(t("verifiedRequired"))+'"':'')+' aria-pressed="'+after+'">'+escapeHtml(after?t("afterSelected"):t("setAfter"))+'</button></div></article>';
    }).join("");
  }
  el.beforeLabel.textContent=selectedSessionLabel(state.beforeId);el.afterLabel.textContent=selectedSessionLabel(state.afterId);
}
function selectedSessionLabel(sessionId){const entry=state.history.find(function(item){return item.session_id===sessionId;});return entry?historyLabel(entry):t("notSelected");}
function updateCompareButton() {
  const before=state.history.find(function(item){return item.session_id===state.beforeId;});
  const after=state.history.find(function(item){return item.session_id===state.afterId;});
  const scenario=document.querySelector('input[name="same-scenario"]:checked');
  let enabled=true, reason="";
  if(!before||!after||!state.beforeId||!state.afterId||state.beforeId===state.afterId){enabled=false;reason=t("compareSelectHint");}
  else if(!before.verified||!after.verified){enabled=false;reason=t("verifiedRequired");}
  else if(normApp(before.application)!==normApp(after.application)){enabled=false;reason=t("sameApplication");}
  else if(!scenario){enabled=false;reason=t("scenarioRequired");}
  el.runCompare.disabled=!enabled;el.compareGuidance.textContent=reason;
}
async function performCompare() {
  updateCompareButton();if(el.runCompare.disabled||!invoke)return;
  const before=state.history.find(function(item){return item.session_id===state.beforeId;});
  const after=state.history.find(function(item){return item.session_id===state.afterId;});
  const choice=document.querySelector('input[name="same-scenario"]:checked');
  const sameScenario=Boolean(choice && choice.value==="yes");
  try{
    const result=await invoke("session_compare",{baselineSessionId:before.session_id,candidateSessionId:after.session_id,sameScenario:sameScenario});
    renderComparison(result);
  }catch(_){showToast(t("compareFailed"));}
}
function renderComparison(result) {
  const status=String(result.comparability||"");
  let title,body;
  if(status==="Comparable"){title=t("compareComparable");body=t("compareComparableBody");}
  else if(status==="LimitedSampleEquivalence"){title=t("compareLimited");body=t("compareLimitedBody");}
  else{title=t("compareInconclusive");body=t("compareInconclusiveBody");}
  el.compareStatusTitle.textContent=title;el.compareStatusBody.textContent=body+(status==="InconclusiveDifferentScenario"?" "+t("compareAdvice"):"");
  const metrics=[[t("expertP99"),result.p99_change],[t("expertAverageFps"),result.average_fps_change],[t("expertStutters"),result.stutter_change]];
  el.compareMetrics.innerHTML=metrics.map(function(pair){return "<div><dt>"+escapeHtml(pair[0])+"</dt><dd>"+escapeHtml(pair[1]===undefined||pair[1]===null?t("noData"):pair[1])+"</dd></div>";}).join("");
  el.compareMetrics.hidden = status === "InconclusiveDifferentScenario";
  el.compareResult.hidden=false;
}
async function openHistoryEntry(sessionId) {
  try{
    const snapshot=await invoke("session_history_load",{sessionId:sessionId});
    clearPolling();state.stopRequested=false;state.snapshot=snapshot;
    if(snapshot.status==="completed")renderCompleted(snapshot);else await showFailureFromMessage(snapshot.message||"");
  }catch(error){await showFailureFromMessage(String(error));}
}
function loadSettings() {
  if(el.presentmonPath)el.presentmonPath.value=state.presentmonPath;
  if(el.duration)el.duration.value=String(state.durationSeconds);
  if(el.interval)el.interval.value=String(state.intervalMs);
  if(el.excludeDropped)el.excludeDropped.checked=state.excludeDropped;
  if(el.noProcessProbing)el.noProcessProbing.checked=state.noProcessProbing;
  if(el.languageSetting)el.languageSetting.value=state.languagePreference;
  syncPreferenceControls();
}
function saveSettings() {
  const duration=Number(el.duration.value),interval=Number(el.interval.value);
  if(!Number.isInteger(duration)||duration<1||duration>3600||!Number.isInteger(interval)||interval<10||interval>5000){el.settingsMessage.textContent=t("invalidAdvanced");return;}
  state.durationSeconds=duration;state.intervalMs=interval;state.excludeDropped=el.excludeDropped.checked;state.noProcessProbing=el.noProcessProbing.checked;state.presentmonPath=String(el.presentmonPath.value||"").trim();
  state.notifyStable=Array.from(document.querySelectorAll("[data-notify-stable]")).some(function(input){return input.checked;});
  state.languagePreference=el.languageSetting.value;state.locale=detectLocale(state.languagePreference);
  storageSet("rwp.language",state.languagePreference);storageSet("rwp.durationSeconds",state.durationSeconds);storageSet("rwp.intervalMs",state.intervalMs);storageSet("rwp.excludeDropped",state.excludeDropped);storageSet("rwp.noProcessProbing",state.noProcessProbing);storageSet("rwp.presentmonPath",state.presentmonPath);storageSet("rwp.notifyStable",state.notifyStable);
  applyLocale();loadSettings();el.settingsMessage.textContent=t("settingsSaved");refreshPreflight();
}
function openRepair(kind) {
  if(kind==="permission"){el.repairTitle.textContent=t("permissionRepairTitle");el.repairBody.innerHTML=t("permissionRepairBody");}
  else{el.repairTitle.textContent=t("componentRepairTitle");el.repairBody.innerHTML=t("componentRepairBody");}
  if(!el.repairDialog.open)el.repairDialog.showModal();
}
function cancelCountdown() {
  if(state.flow!=="countdown")return;
  stopCountdown();state.flow="idle";state.startRequested=false;setView("home");
}
function handleFailureAction() {
  if(state.lastErrorKind==="permission"){openRepair("permission");return;}
  if(["component","presentmon_name","version"].includes(state.lastErrorKind)){choosePresentMon();return;}
  if(state.lastErrorKind==="target"){setView("home");state.processId="";el.processSelect.value="";refreshProcesses();return;}
  setView("home");beginCountdown();
}
async function copyTechnical() {
  const text=String(state.lastError||"");
  try{
    if(navigator.clipboard&&navigator.clipboard.writeText)await navigator.clipboard.writeText(text);
    else{const temp=document.createElement("textarea");temp.value=text;temp.style.position="fixed";temp.style.opacity="0";document.body.appendChild(temp);temp.select();const ok=document.execCommand("copy");temp.remove();if(!ok)throw new Error("copy failed");}
    showToast(t("unknownCopy"));
  }catch(_){showToast(t("copyFailed"));}
}
async function onCloseRequested() {
  let snapshot=state.snapshot;
  if(invoke){try{snapshot=await invoke("session_status");}catch(_){}}
  if((snapshot&&snapshot.status==="running")||["countdown","recording","analyzing"].includes(state.flow)){
    if(!el.closeDialog.open)el.closeDialog.showModal();
  }else await finalizeWindowClose();
}
async function confirmWindowClose() {
  el.closeDialog.close();state.closeAfterTerminal=true;
  if(state.flow==="countdown"){stopCountdown();state.flow="idle";}
  if(state.snapshot&&state.snapshot.status==="running"&&!state.stopCallSent){
    state.stopRequested=true;state.stopCallSent=true;showMeasureStage("analyzing");
    try{await invoke("session_stop");}catch(_){}
    if(!state.pollTimer)startPolling();return;
  }
  if(state.snapshot&&state.snapshot.status==="running"){if(!state.pollTimer)startPolling();return;}
  await finalizeWindowClose();
}
async function finalizeWindowClose() {
  if(!invoke)return;
  try{await invoke("session_close_confirmed");}
  catch(_){state.closeAfterTerminal=false;showToast(t("preflightFailed"));}
}
function wireEvents() {
  el.nav.forEach(function(button) {
    button.addEventListener("click", function(event) {
      event.preventDefault();
      if (["countdown", "recording", "analyzing"].includes(state.flow)) return;
      setView(button.getAttribute("data-view"));
    });
  });
  el.processSelect.addEventListener("change",async function(){state.processId=el.processSelect.value;await refreshPreflight();});
  $("refresh-processes").addEventListener("click",refreshProcesses);
  el.start.addEventListener("click",beginCountdown);
  $("cancel-countdown").addEventListener("click",cancelCountdown);
  el.stopMeasurement.addEventListener("click",function(){if(state.flow==="recording"&&!state.stopCallSent&&!el.stopDialog.open)el.stopDialog.showModal();});
  $("continue-measuring").addEventListener("click",function(){el.stopDialog.close();});
  $("confirm-stop").addEventListener("click",async function(){
    el.stopDialog.close();if(!invoke||state.stopCallSent||!state.snapshot||state.snapshot.status!=="running")return;
    state.stopRequested=true;state.stopCallSent=true;showMeasureStage("analyzing");
    try{await invoke("session_stop");}catch(_){showToast(t("preflightFailed"));}
    if(!state.pollTimer)startPolling();
  });
  el.permissionFix.addEventListener("click",function(){openRepair("permission");});
  el.componentSetup.addEventListener("click",choosePresentMon);
  $("repair-done").addEventListener("click",function(){el.repairDialog.close();});
  $("browse-presentmon-settings").addEventListener("click",choosePresentMon);
  $("save-settings").addEventListener("click",saveSettings);
  el.languageSetting.addEventListener("change",function(){state.languagePreference=el.languageSetting.value;state.locale=detectLocale(state.languagePreference);storageSet("rwp.language",state.languagePreference);applyLocale();});
  document.querySelectorAll("[data-prep-choice]").forEach(function(input){input.addEventListener("change",function(){if(!input.checked)return;state.prepSeconds=Number(input.value);storageSet("rwp.prepSeconds",state.prepSeconds);syncPreferenceControls();});});
  document.querySelectorAll("[data-notify-stable]").forEach(function(input){input.addEventListener("change",function(){state.notifyStable=Boolean(input.checked);storageSet("rwp.notifyStable",state.notifyStable);syncPreferenceControls();});});
  $("refresh-history").addEventListener("click",refreshHistory);
  el.historyList.addEventListener("click",async function(event){
    const button=event.target.closest("button[data-history-action]");
    if(!button){const nav=event.target.closest("[data-view]");if(nav)setView(nav.getAttribute("data-view"));return;}
    if(button.disabled)return;
    const id=button.getAttribute("data-session-id"),action=button.getAttribute("data-history-action");
    if(action==="load"){await openHistoryEntry(id);return;}
    const entry=state.history.find(function(item){return item.session_id===id;});if(!entry||!entry.verified)return;
    const oldBefore = state.beforeId, oldAfter = state.afterId;
    if(action==="before")state.beforeId=id;if(action==="after")state.afterId=id;
    if(state.beforeId&&state.beforeId===state.afterId){if(action==="before")state.afterId="";else state.beforeId="";}
    if (oldBefore !== state.beforeId || oldAfter !== state.afterId) {
      document.querySelectorAll('input[name="same-scenario"]').forEach(function(input) {
        input.checked = false;
      });
      el.compareResult.hidden = true;
    }
    renderHistory();updateCompareButton();
  });
  document.querySelectorAll('input[name="same-scenario"]').forEach(function(input){input.addEventListener("change",updateCompareButton);});
  el.runCompare.addEventListener("click",performCompare);
  el.resultCompare.addEventListener("click",function(){setView("history");});
  $("measure-again").addEventListener("click",async function(){
    setView("home");const name=state.snapshot&&state.snapshot.application;
    await refreshProcesses();
    const found=state.processes.find(function(row){return normApp(row.name)===normApp(name);});
    if(found){state.processId=String(found.process_id);el.processSelect.value=state.processId;await refreshPreflight();beginCountdown();}
  });
  el.retryAction.addEventListener("click",handleFailureAction);
  $("copy-technical").addEventListener("click",copyTechnical);
  $("stay-in-app").addEventListener("click",function(){state.closeAfterTerminal=false;el.closeDialog.close();});
  $("confirm-close-app").addEventListener("click",confirmWindowClose);
  if(tauriEvent&&tauriEvent.listen){tauriEvent.listen("rwp-window-close-requested",onCloseRequested).catch(function(){});}
  document.addEventListener("visibilitychange",function(){if(state.flow==="countdown")tickCountdown();if(state.flow==="recording"&&state.snapshot)updateProgress(state.snapshot);});
  window.addEventListener("focus",function(){if(state.flow==="countdown")tickCountdown();if(state.flow==="recording"&&state.snapshot)updateProgress(state.snapshot);});
}
function initialize() {
  applyLocale();wireEvents();loadSettings();refreshProcesses();refreshHistory();
}
initialize();
