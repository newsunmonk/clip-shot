<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { save } from "@tauri-apps/plugin-dialog";
  import {
    type Settings,
    type Shortcuts,
    type HistoryEntry,
    type CaptureMode,
    captureAllDisplays,
    openCaptureOverlay,
    listHistory,
    historyThumb,
    historyFull,
    copyHistory,
    deleteHistory,
    clearHistory,
    exportHistory,
    defaultExportName,
    getSettings,
    saveSettings,
    showMain,
    screenPermission,
    requestScreenPermission,
    openScreenSettings,
    openKeyboardSettings,
    restartApp,
    setNativeScreenshots,
    autostartStatus,
  } from "./ipc";
  import { t, modeName, type Lang } from "./i18n";

  let lang = $state<Lang>("ko");
  let tab = $state<"history" | "settings">("history");
  let settings = $state<Settings | null>(null);
  let entries = $state<HistoryEntry[]>([]);
  let thumbs = $state<Record<string, string>>({});

  let toastMsg = $state("");
  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  let flashOn = $state(false);

  let preview = $state<{ id: string; src: string } | null>(null);
  let previewScale = $state(1);
  let metaHeld = $state(false);
  let savedFlash = $state(false);
  let settingsError = $state("");
  let recording = $state<keyof Shortcuts | null>(null);
  let hasPermission = $state(true);

  const tr = (k: string) => t(lang, k);

  // 단축키 문자열을 ⌘⇧4 처럼 보기 좋게.
  function fmtAccel(a: string): string {
    return a
      .replace(/CmdOrCtrl/gi, "⌘")
      .replace(/Command|Cmd|Super|Meta/gi, "⌘")
      .replace(/Control|Ctrl/gi, "⌃")
      .replace(/Option|Alt/gi, "⌥")
      .replace(/Shift/gi, "⇧")
      .replace(/\+/g, "");
  }

  function resolveTheme(theme: string): string {
    if (theme === "light" || theme === "dark") return theme;
    return matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  function applyTheme(theme: string) {
    document.documentElement.dataset.theme = resolveTheme(theme);
  }

  // 언어/테마는 설정값이 바뀌는 즉시 반영(저장 전에도).
  $effect(() => {
    if (settings) lang = settings.language;
  });
  $effect(() => {
    if (settings) applyTheme(settings.theme);
  });

  async function disableNative() {
    await setNativeScreenshots(false);
    showToast(tr("settings.nativeApplied"));
  }
  async function restoreNative() {
    await setNativeScreenshots(true);
    showToast(tr("settings.nativeApplied"));
  }

  async function checkPermission() {
    try {
      // 캡처가 성공한 적 있으면(히스토리 존재) 권한은 확실히 있는 것 — 잘못된 false 무시.
      hasPermission = (await screenPermission()) || entries.length > 0;
    } catch {
      hasPermission = true;
    }
  }
  async function requestPermission() {
    try {
      await requestScreenPermission();
    } catch {
      /* 무시 */
    }
    await checkPermission();
  }

  function showToast(msg: string) {
    toastMsg = msg;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toastMsg = ""), 1800);
  }

  function playShutter() {
    if (!settings?.shutterSound) return;
    try {
      const ctx = new (window.AudioContext || (window as any).webkitAudioContext)();
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.frequency.value = 1100;
      gain.gain.setValueAtTime(0.0001, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.2, ctx.currentTime + 0.01);
      gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.12);
      osc.start();
      osc.stop(ctx.currentTime + 0.13);
      osc.onended = () => ctx.close();
    } catch {
      /* 무시 */
    }
  }

  function flash() {
    if (!settings?.flashEffect) return;
    flashOn = true;
    setTimeout(() => (flashOn = false), 160);
  }

  async function loadHistory() {
    entries = await listHistory();
    const map: Record<string, string> = {};
    await Promise.all(
      entries.map(async (e) => {
        try {
          map[e.id] = await historyThumb(e.id);
        } catch {
          /* 썸네일 없음 */
        }
      }),
    );
    thumbs = map;
  }

  async function doCapture(mode: CaptureMode) {
    try {
      if (mode === "all") {
        await captureAllDisplays();
      } else {
        // 영역/창/디스플레이는 모두 오버레이에서 호버→클릭으로 처리(앱 창 안 띄움).
        await openCaptureOverlay(mode);
      }
    } catch (e) {
      showToast(`${tr("error.prefix")}: ${tr("error.permission")}`);
      console.error(e);
    }
  }

  async function openPreview(id: string, e?: MouseEvent) {
    // 돋보기 커서가 클릭 후 잔상으로 남지 않도록, 클릭한 요소의 커서를 즉시 기본으로.
    if (e?.currentTarget instanceof HTMLElement) e.currentTarget.style.cursor = "default";
    try {
      const src = await historyFull(id);
      previewScale = 1;
      metaHeld = false;
      preview = { id, src };
    } catch (err) {
      showToast(`${tr("error.prefix")}: ${err}`);
    }
  }

  function zoomIn() {
    previewScale = Math.min(previewScale * 1.25, 8);
  }
  function zoomOut() {
    previewScale = Math.max(previewScale / 1.25, 0.2);
  }
  function zoomReset() {
    previewScale = 1;
  }

  async function onExport(id: string) {
    const ext = settings?.imageFormat ?? "png";
    const name = await defaultExportName(id);
    const path = await save({ defaultPath: name, filters: [{ name: "Image", extensions: [ext] }] });
    if (!path) return;
    await exportHistory(id, path);
    showToast(tr("toast.exported"));
  }

  async function onCopy(id: string) {
    await copyHistory(id);
    showToast(tr("toast.copied"));
  }

  async function onDelete(id: string) {
    await deleteHistory(id);
    if (preview?.id === id) preview = null;
    showToast(tr("toast.deleted"));
    await loadHistory();
  }

  async function onClear() {
    if (!confirm(tr("history.clearConfirm"))) return;
    await clearHistory();
    await loadHistory();
  }

  async function onSaveSettings() {
    if (!settings) return;
    settingsError = "";
    try {
      await saveSettings($state.snapshot(settings) as Settings);
      lang = settings.language;
      savedFlash = true;
      setTimeout(() => (savedFlash = false), 1500);
    } catch (e) {
      settingsError = String(e);
    }
  }

  // ── 단축키 레코더 ──
  function accelFromEvent(e: KeyboardEvent): string | null {
    const mods: string[] = [];
    if (e.metaKey || e.ctrlKey) mods.push("CmdOrCtrl");
    if (e.altKey) mods.push("Alt");
    if (e.shiftKey) mods.push("Shift");
    const c = e.code;
    if (
      ["MetaLeft", "MetaRight", "ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight"].includes(
        c,
      )
    )
      return null;
    let key = "";
    if (c.startsWith("Key")) key = c.slice(3);
    else if (c.startsWith("Digit")) key = c.slice(5);
    else if (c.startsWith("Arrow")) key = c.slice(5);
    else key = c;
    if (!key || mods.length === 0) return null;
    return [...mods, key].join("+");
  }

  function startRecord(field: keyof Shortcuts) {
    recording = field;
  }

  function onRecordKey(e: KeyboardEvent) {
    if (!recording || !settings) return;
    e.preventDefault();
    if (e.key === "Escape") {
      recording = null;
      return;
    }
    const accel = accelFromEvent(e);
    if (accel) {
      settings.shortcuts[recording] = accel;
      recording = null;
    }
  }

  function onKeyup(e: KeyboardEvent) {
    metaHeld = e.metaKey || e.ctrlKey;
  }

  function onKeydown(e: KeyboardEvent) {
    if (preview) {
      metaHeld = e.metaKey || e.ctrlKey;
      if (e.key === "Escape") {
        preview = null;
        return;
      }
      if (e.metaKey || e.ctrlKey) {
        if (e.key === "=" || e.key === "+") {
          e.preventDefault();
          zoomIn();
        } else if (e.key === "-" || e.key === "_") {
          e.preventDefault();
          zoomOut();
        } else if (e.key === "0") {
          e.preventDefault();
          zoomReset();
        }
      }
      return;
    }
    if (recording) onRecordKey(e);
  }

  function fmtTime(ms: number): string {
    return new Date(ms).toLocaleString(lang === "ko" ? "ko-KR" : "en-US");
  }

  const captureButtons: { mode: CaptureMode; key: string }[] = [
    { mode: "region", key: "mode.region" },
    { mode: "window", key: "mode.window" },
    { mode: "display", key: "mode.display" },
    { mode: "all", key: "mode.all" },
  ];

  const shortcutFields: { key: keyof Shortcuts; label: string }[] = [
    { key: "region", label: "mode.region" },
    { key: "window", label: "mode.window" },
    { key: "display", label: "mode.display" },
    { key: "allDisplays", label: "mode.all" },
    { key: "history", label: "shortcut.history" },
  ];

  onMount(async () => {
    settings = await getSettings();
    // 시작 프로그램 등록은 실제 상태를 읽어 토글에 반영.
    try {
      settings.autostart = await autostartStatus();
    } catch {
      /* 무시 */
    }
    lang = settings.language;
    applyTheme(settings.theme);
    matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
      if (settings?.theme === "system") applyTheme("system");
    });
    await loadHistory();
    await checkPermission();

    await listen<HistoryEntry>("captured", () => {
      hasPermission = true; // 캡처 성공 = 권한 있음
      flash();
      playShutter();
      if (settings?.showToast) showToast(tr("toast.captured"));
      loadHistory();
    });
    await listen<string>("hotkey-action", (e) => {
      const a = e.payload;
      if (a === "region" || a === "window" || a === "display") doCapture(a as CaptureMode);
      else if (a === "all_displays") doCapture("all");
      else if (a === "history") {
        showMain();
        tab = "history";
      }
    });
    await listen<string>("navigate", (e) => {
      tab = e.payload === "settings" ? "settings" : "history";
    });
  });
</script>

<svelte:window onkeydown={onKeydown} onkeyup={onKeyup} />

<div class="app">
  <header data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <h1>{tr("app.title")}</h1>
      <span class="sub">{tr("app.subtitle")}</span>
    </div>
    <div class="capbar">
      {#each captureButtons as b (b.mode)}
        <button class="cap" onclick={() => doCapture(b.mode)}>{tr(b.key)}</button>
      {/each}
    </div>
  </header>

  {#if !hasPermission}
    <div class="permbanner">
      <div class="permtext">
        <strong>{tr("perm.title")}</strong>
        <span>{tr("perm.desc")}</span>
      </div>
      <div class="permbtns">
        <button onclick={requestPermission}>{tr("perm.request")}</button>
        <button onclick={openScreenSettings}>{tr("perm.settings")}</button>
        <button onclick={checkPermission}>{tr("perm.recheck")}</button>
        <button class="primary" onclick={restartApp}>{tr("perm.restart")}</button>
      </div>
    </div>
  {/if}

  <nav class="tabs">
    <button class:active={tab === "history"} onclick={() => (tab = "history")}>{tr("tab.history")}</button>
    <button class:active={tab === "settings"} onclick={() => (tab = "settings")}>{tr("tab.settings")}</button>
  </nav>

  {#if tab === "history"}
    <section>
      <div class="hbar">
        <span class="muted">{entries.length} {tr("history.count")}</span>
        <div class="spacer"></div>
        <button class="ghost" onclick={loadHistory}>{tr("common.refresh")}</button>
        {#if entries.length > 0}
          <button class="ghost danger" onclick={onClear}>{tr("history.clear")}</button>
        {/if}
      </div>

      {#if entries.length === 0}
        <p class="empty">{tr("history.empty")}</p>
      {:else}
        <div class="grid">
          {#each entries as e (e.id)}
            <div class="card">
              <button class="thumbwrap" onclick={(ev) => openPreview(e.id, ev)} aria-label="preview">
                {#if thumbs[e.id]}
                  <img src={thumbs[e.id]} alt={modeName(lang, e.mode)} />
                {/if}
              </button>
              <div class="meta">
                <span class="tag">{modeName(lang, e.mode)}</span>
                <span class="dims">{e.width}×{e.height}</span>
              </div>
              <div class="time">{fmtTime(e.createdAt)}</div>
              <div class="actions">
                <button onclick={() => onCopy(e.id)}>{tr("history.copy")}</button>
                <button onclick={() => onExport(e.id)}>{tr("history.export")}</button>
                <button class="danger" onclick={() => onDelete(e.id)}>{tr("history.delete")}</button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  {:else if settings}
    <section class="settings">
      <h2>{tr("settings.general")}</h2>
      <label class="row"><span>{tr("settings.autostart")}</span><input type="checkbox" bind:checked={settings.autostart} /></label>
      <label class="row"><span>{tr("settings.hotkeysEnabled")}</span><input type="checkbox" bind:checked={settings.hotkeysEnabled} /></label>
      <p class="muted small">{tr("settings.hotkeyNote")}</p>

      <h2>{tr("settings.effects")}</h2>
      <label class="row"><span>{tr("settings.showToast")}</span><input type="checkbox" bind:checked={settings.showToast} /></label>
      <label class="row"><span>{tr("settings.shutterSound")}</span><input type="checkbox" bind:checked={settings.shutterSound} /></label>
      <label class="row"><span>{tr("settings.flashEffect")}</span><input type="checkbox" bind:checked={settings.flashEffect} /></label>

      <h2>{tr("settings.storage")}</h2>
      <label class="row">
        <span>{tr("settings.imageFormat")}</span>
        <select bind:value={settings.imageFormat}><option value="png">PNG</option><option value="jpg">JPG</option></select>
      </label>
      <label class="row">
        <span>{tr("settings.language")}</span>
        <select bind:value={settings.language}><option value="ko">한국어</option><option value="en">English</option></select>
      </label>
      <label class="row">
        <span>{tr("settings.theme")}</span>
        <select bind:value={settings.theme}>
          <option value="system">{tr("theme.system")}</option>
          <option value="light">{tr("theme.light")}</option>
          <option value="dark">{tr("theme.dark")}</option>
        </select>
      </label>

      <h2>{tr("settings.shortcuts")}</h2>
      <p class="muted small">{tr("settings.shortcutHint")}</p>
      {#each shortcutFields as f (f.key)}
        <div class="row">
          <span>{tr(f.label)}</span>
          <button
            class="keybtn"
            class:recording={recording === f.key}
            onclick={() => startRecord(f.key)}
          >
            {recording === f.key ? tr("shortcut.recording") : fmtAccel(settings.shortcuts[f.key])}
          </button>
        </div>
      {/each}

      <h2>{tr("settings.nativeShortcuts")}</h2>
      <p class="muted small">{tr("settings.nativeNote")}</p>
      <div class="btnrow">
        <button class="ghost" onclick={disableNative}>{tr("settings.nativeDisable")}</button>
        <button class="ghost" onclick={restoreNative}>{tr("settings.nativeRestore")}</button>
        <button class="ghost" onclick={openKeyboardSettings}>{tr("settings.openKeyboard")}</button>
      </div>

      {#if settingsError}<p class="err">{settingsError}</p>{/if}
      <div class="saverow">
        <button class="primary" onclick={onSaveSettings}>{tr("settings.save")}</button>
        {#if savedFlash}<span class="ok">✓ {tr("settings.saved")}</span>{/if}
      </div>
    </section>
  {/if}
</div>

<!-- 큰 미리보기 -->
{#if preview}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal dark" role="presentation" onclick={() => (preview = null)}>
    <div class="previewbox" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()}>
      <div class="pvscroll">
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="pvinner"
          class:zoomout={metaHeld}
          role="presentation"
          style="width: {previewScale * 100}%;"
          onclick={(e) => (e.metaKey || e.ctrlKey ? zoomOut() : zoomIn())}
        >
          <img src={preview.src} alt="preview" draggable="false" />
        </div>
      </div>
      <div class="pvactions">
        <button onclick={zoomOut} aria-label="zoom out">−</button>
        <button onclick={zoomReset}>{Math.round(previewScale * 100)}%</button>
        <button onclick={zoomIn} aria-label="zoom in">+</button>
        <span class="pvsep"></span>
        <button onclick={() => preview && onCopy(preview.id)}>{tr("preview.copy")}</button>
        <button onclick={() => preview && onExport(preview.id)}>{tr("preview.export")}</button>
        <button class="ghost" onclick={() => (preview = null)}>{tr("preview.close")}</button>
      </div>
    </div>
  </div>
{/if}

{#if toastMsg}<div class="toast">{toastMsg}</div>{/if}
{#if flashOn}<div class="screenflash"></div>{/if}

<style>
  :global(:root) {
    --bg: #f4f4f3;
    --surface: #ffffff;
    --surface-2: #fafafa;
    --border: #e4e4e2;
    --text: #1d1d1f;
    --muted: #8a8a87;
    --accent: #2c2c2e;
    --on-accent: #ffffff;
    --tag-bg: #efefec;
    --tag-fg: #55554f;
    --danger: #b23b3b;
  }
  :global(:root[data-theme="dark"]) {
    --bg: #1a1a1c;
    --surface: #242426;
    --surface-2: #2b2b2e;
    --border: #343437;
    --text: #e8e8e6;
    --muted: #909090;
    --accent: #e8e8e6;
    --on-accent: #1a1a1c;
    --tag-bg: #343437;
    --tag-fg: #c8c8c4;
    --danger: #e07070;
  }
  :global(html) {
    background: var(--bg);
  }
  :global(html),
  :global(body) {
    height: 100%;
  }
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Apple SD Gothic Neo",
      "Malgun Gothic", sans-serif;
    color: var(--text);
    background: var(--bg);
    -webkit-font-smoothing: antialiased;
    -webkit-user-select: none;
    user-select: none;
    cursor: default;
  }
  :global(img) {
    -webkit-user-drag: none;
    user-select: none;
  }
  .app {
    max-width: 900px;
    margin: 0 auto;
    padding: 34px 24px 48px;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 14px;
    justify-content: space-between;
  }
  .brand h1 {
    margin: 0;
    font-size: 21px;
    font-weight: 650;
    letter-spacing: -0.2px;
  }
  .sub {
    color: var(--muted);
    font-size: 12.5px;
  }
  .capbar {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .cap {
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 13px;
    font-size: 13px;
    font-weight: 550;
    cursor: pointer;
    transition: background 0.12s, border-color 0.12s;
  }
  .cap:hover {
    background: var(--surface-2);
    border-color: #d2d2cf;
  }
  .cap:active {
    background: #efefed;
  }
  .permbanner {
    margin-top: 16px;
    padding: 12px 14px;
    border: 1px solid #e6d3a8;
    background: #fbf4e2;
    border-radius: 10px;
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  .permtext {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 220px;
  }
  .permtext strong {
    font-size: 13.5px;
    color: #7a5c12;
  }
  .permtext span {
    font-size: 12.5px;
    color: #8a7330;
  }
  .permbtns {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .permbtns button {
    font-size: 12.5px;
    padding: 7px 12px;
    border: 1px solid #e0cf9e;
    background: #fff;
    border-radius: 7px;
    cursor: pointer;
    color: #5f4a14;
  }
  .permbtns button.primary {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }
  .tabs {
    display: flex;
    gap: 2px;
    margin: 20px 0 16px;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    background: none;
    border: none;
    padding: 9px 12px;
    font-size: 13.5px;
    color: var(--muted);
    cursor: pointer;
    border-bottom: 1.5px solid transparent;
    margin-bottom: -1px;
  }
  .tabs button.active {
    color: var(--text);
    border-bottom-color: var(--accent);
    font-weight: 600;
  }
  .hbar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 14px;
  }
  .spacer {
    flex: 1;
  }
  .muted {
    color: var(--muted);
    font-size: 12.5px;
  }
  .small {
    font-size: 12px;
    margin: 2px 0 10px;
  }
  .empty {
    color: var(--muted);
    text-align: center;
    padding: 64px 0;
    font-size: 13.5px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(196px, 1fr));
    gap: 13px;
  }
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 11px;
    padding: 9px;
  }
  .thumbwrap {
    display: block;
    width: 100%;
    aspect-ratio: 16/10;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 7px;
    overflow: hidden;
    cursor: zoom-in;
    padding: 0;
  }
  .thumbwrap img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 8px;
  }
  .tag {
    background: var(--tag-bg);
    color: var(--tag-fg);
    font-size: 11.5px;
    padding: 2px 8px;
    border-radius: 6px;
  }
  .dims {
    color: var(--muted);
    font-size: 11.5px;
  }
  .time {
    color: var(--muted);
    font-size: 11px;
    margin: 4px 0 9px;
  }
  .actions {
    display: flex;
    gap: 5px;
  }
  .actions button {
    flex: 1;
    font-size: 12px;
    padding: 6px 0;
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 6px;
    cursor: pointer;
    color: var(--text);
  }
  .actions button:hover {
    background: var(--surface-2);
  }
  .danger {
    color: var(--danger);
  }
  .ghost {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 7px 12px;
    font-size: 12.5px;
    cursor: pointer;
    color: var(--text);
  }
  .ghost:hover {
    background: var(--surface-2);
  }
  .settings h2 {
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 22px 0 6px;
    color: var(--muted);
    font-weight: 600;
  }
  .settings .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    font-size: 13.5px;
    border-bottom: 1px solid #efefed;
  }
  .settings .row > span {
    flex: 1;
  }
  .settings .row input:not([type]),
  .settings .row select {
    padding: 6px 9px;
    border: 1px solid var(--border);
    border-radius: 7px;
    font-size: 13px;
    background: var(--surface);
    color: var(--text);
    min-width: 120px;
  }
  .settings .row input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }
  .keybtn {
    min-width: 170px;
    text-align: center;
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--surface-2);
    font:
      12.5px/1.2 ui-monospace,
      SFMono-Regular,
      monospace;
    color: var(--text);
    cursor: pointer;
  }
  .keybtn:hover {
    border-color: #c9c9c6;
  }
  .keybtn.recording {
    border-color: var(--accent);
    background: #ecebff00;
    box-shadow: 0 0 0 2px rgba(44, 44, 46, 0.12);
    color: var(--muted);
  }
  .saverow {
    margin-top: 20px;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border: none;
    border-radius: 8px;
    padding: 9px 22px;
    font-weight: 600;
    font-size: 13.5px;
    cursor: pointer;
  }
  .primary:hover {
    opacity: 0.88;
  }
  .btnrow {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .ok {
    color: #3a7d44;
    font-size: 12.5px;
  }
  .err {
    color: var(--danger);
    font-size: 12.5px;
    margin-top: 10px;
  }
  .modal {
    position: fixed;
    inset: 0;
    background: rgba(20, 20, 22, 0.32);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
    backdrop-filter: blur(2px);
  }
  .modal.dark {
    background: rgba(10, 10, 12, 0.78);
  }
  .previewbox {
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: center;
    max-width: 92vw;
    max-height: 92vh;
  }
  .pvscroll {
    max-width: 92vw;
    max-height: 80vh;
    overflow: auto;
    text-align: center;
    background: transparent;
  }
  .pvinner {
    display: inline-block;
    min-width: 20%;
    cursor: zoom-in;
    font-size: 0;
  }
  .pvinner.zoomout {
    cursor: zoom-out;
  }
  .previewbox img {
    display: block;
    width: 100%;
    height: auto;
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  }
  .pvactions {
    display: flex;
    align-items: center;
    gap: 4px;
    background: rgba(30, 30, 32, 0.72);
    backdrop-filter: blur(12px);
    padding: 6px;
    border-radius: 13px;
    border: 1px solid rgba(255, 255, 255, 0.12);
  }
  .pvsep {
    width: 1px;
    align-self: stretch;
    background: rgba(255, 255, 255, 0.16);
    margin: 2px 4px;
  }
  .pvactions button {
    background: transparent;
    color: #f2f2f2;
    border: none;
    border-radius: 8px;
    padding: 8px 16px;
    font-size: 13px;
    cursor: pointer;
    min-width: 38px;
  }
  .pvactions button:hover {
    background: rgba(255, 255, 255, 0.14);
  }
  .toast {
    position: fixed;
    bottom: 26px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(28, 28, 30, 0.94);
    color: #f4f4f3;
    padding: 10px 20px;
    border-radius: 999px;
    font-size: 13.5px;
    z-index: 60;
  }
  .screenflash {
    position: fixed;
    inset: 0;
    background: #fff;
    opacity: 0.7;
    pointer-events: none;
    z-index: 70;
    animation: fade 0.16s ease-out forwards;
  }
  @keyframes fade {
    to {
      opacity: 0;
    }
  }
</style>
