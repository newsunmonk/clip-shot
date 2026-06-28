<script lang="ts">
  // 통합 캡처 오버레이(모니터별).
  //  - region : 화면 전체 어둡게 + 십자선 + 드래그 영역만 밝게 + 돋보기 루페(픽셀 단위)
  //  - window : 화면 어둡게 + 마우스 올린 창에 강조 테두리 + 카메라 아이콘 + 앱 이름
  //  - display: 화면 어둡게 + 마우스 올린 디스플레이에 카메라 아이콘
  // 실제 픽셀은 Rust가 표시 직전 고정(freeze)해 둔 것에서 잘라낸다(오버레이 딤이 안 섞임).
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    type WindowRect,
    captureRegion,
    cancelRegion,
    listWindowRects,
    commitWindow,
    commitDisplay,
    overlayBg,
  } from "./ipc";

  let mode = $state<"region" | "window" | "display">("region");
  let displayId = 0;
  let originX = $state(0);
  let originY = $state(0);
  let scale = $state(1);
  let bg = $state<string | null>(null);

  let dragging = false;
  let startX = 0;
  let startY = 0;
  let cur = $state({ x: 0, y: 0, inside: false });
  let sel = $state<{ x: number; y: number; w: number; h: number } | null>(null);

  let rects: WindowRect[] = [];
  let hovered = $state<WindowRect | null>(null);

  const LOUPE = 150;
  const ZOOM = 8;

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") cancelRegion();
  }

  onMount(async () => {
    const win = getCurrentWindow();
    const parts = win.label.split("-"); // overlay-<mode>-<id>
    mode = (parts[1] as typeof mode) ?? "region";
    displayId = parseInt(parts[2] ?? "0", 10);
    try {
      const pos = await win.outerPosition();
      scale = await win.scaleFactor();
      originX = pos.x / scale;
      originY = pos.y / scale;
    } catch {
      scale = 1;
    }
    if (mode === "region") {
      overlayBg(displayId)
        .then((b) => (bg = b))
        .catch(() => (bg = null));
    } else if (mode === "window") {
      try {
        rects = await listWindowRects();
      } catch {
        rects = [];
      }
    }
    window.addEventListener("keydown", onKey);
  });
  onDestroy(() => window.removeEventListener("keydown", onKey));

  function findWindowAt(gx: number, gy: number): WindowRect | null {
    for (const r of rects) {
      if (gx >= r.x && gx < r.x + r.width && gy >= r.y && gy < r.y + r.height) return r;
    }
    return null;
  }

  function move(e: MouseEvent) {
    cur = { x: e.clientX, y: e.clientY, inside: true };
    if (mode === "window") {
      hovered = findWindowAt(originX + e.clientX, originY + e.clientY);
      return;
    }
    if (mode === "region" && dragging) {
      sel = {
        x: Math.min(startX, e.clientX),
        y: Math.min(startY, e.clientY),
        w: Math.abs(e.clientX - startX),
        h: Math.abs(e.clientY - startY),
      };
    }
  }
  function leave() {
    cur = { ...cur, inside: false };
    hovered = null;
  }
  function onContext(e: MouseEvent) {
    e.preventDefault();
    cancelRegion();
  }
  function down(e: MouseEvent) {
    if (mode !== "region") return;
    dragging = true;
    startX = e.clientX;
    startY = e.clientY;
    sel = { x: startX, y: startY, w: 0, h: 0 };
  }
  async function up() {
    if (mode === "region") {
      if (!dragging) return;
      dragging = false;
      const s = sel;
      if (!s || s.w < 2 || s.h < 2) {
        await cancelRegion();
        return;
      }
      await captureRegion(
        Math.round(originX + s.x),
        Math.round(originY + s.y),
        Math.round(s.w),
        Math.round(s.h),
      );
    } else if (mode === "window") {
      if (hovered) await commitWindow(hovered.id);
      else await cancelRegion();
    } else {
      await commitDisplay(displayId);
    }
  }

  let fullDim = $derived(mode !== "region" || !sel);
  let cameraCursor = $derived(mode !== "region");

  // 돋보기 위치/배경(region)
  let loupeLeft = $derived(
    cur.x + 24 + LOUPE > (typeof window !== "undefined" ? window.innerWidth : 9999)
      ? cur.x - 24 - LOUPE
      : cur.x + 24,
  );
  let loupeTop = $derived(
    cur.y + 24 + LOUPE > (typeof window !== "undefined" ? window.innerHeight : 9999)
      ? cur.y - 24 - LOUPE
      : cur.y + 24,
  );
  let bgSizeW = $derived((typeof window !== "undefined" ? window.innerWidth : 0) * ZOOM);
  let bgPosX = $derived(-(cur.x * ZOOM - LOUPE / 2));
  let bgPosY = $derived(-(cur.y * ZOOM - LOUPE / 2));
</script>

<div
  class="ov"
  class:nocursor={cameraCursor}
  role="presentation"
  onmousedown={down}
  onmousemove={move}
  onmouseup={up}
  onmouseleave={leave}
  oncontextmenu={onContext}
>
  {#if fullDim}
    <div class="dim"></div>
  {/if}

  <!-- region: 십자선(항상) -->
  {#if mode === "region" && cur.inside}
    <div class="vline" style="left:{cur.x}px"></div>
    <div class="hline" style="top:{cur.y}px"></div>
  {/if}

  <!-- region: 선택 영역(바깥 어둡게, 안쪽 라이브 그대로) -->
  {#if mode === "region" && sel}
    <div class="hole" style="left:{sel.x}px; top:{sel.y}px; width:{sel.w}px; height:{sel.h}px">
      <span class="badge">{Math.round(sel.w * scale)} × {Math.round(sel.h * scale)}</span>
    </div>
  {/if}

  <!-- region: 돋보기 루페 -->
  {#if mode === "region" && cur.inside && bg}
    <div
      class="loupe"
      style="left:{loupeLeft}px; top:{loupeTop}px; width:{LOUPE}px; height:{LOUPE}px;
             background-image:url({bg}); background-size:{bgSizeW}px auto;
             background-position:{bgPosX}px {bgPosY}px;"
    >
      <div class="loupe-x"></div>
      <div class="loupe-y"></div>
      <div class="loupe-cell"></div>
      <div class="loupe-coord">
        {Math.round((originX + cur.x) * scale)}, {Math.round((originY + cur.y) * scale)}
      </div>
    </div>
  {/if}

  <!-- window: 호버한 창 강조 + 카메라 -->
  {#if mode === "window" && hovered}
    <div
      class="target"
      style="left:{hovered.x - originX}px; top:{hovered.y - originY}px; width:{hovered.width}px; height:{hovered.height}px;"
    >
      <div class="cam">
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="1.6">
          <rect x="2.5" y="6.5" width="19" height="13" rx="2.5" fill="rgba(20,22,26,0.55)" />
          <path d="M8 6.5 L9.5 4 H14.5 L16 6.5" fill="rgba(20,22,26,0.55)" />
          <circle cx="12" cy="13" r="3.6" />
        </svg>
      </div>
      <span class="badge">{hovered.appName}</span>
    </div>
  {/if}

  <!-- display: 호버 시 카메라 + 안내 -->
  {#if mode === "display" && cur.inside}
    <div class="dcenter">
      <svg width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="1.5">
        <rect x="2.5" y="6.5" width="19" height="13" rx="2.5" fill="rgba(20,22,26,0.55)" />
        <path d="M8 6.5 L9.5 4 H14.5 L16 6.5" fill="rgba(20,22,26,0.55)" />
        <circle cx="12" cy="13" r="3.8" />
      </svg>
      <div class="displaylabel">이 디스플레이 캡처</div>
    </div>
  {/if}

  <div class="hint">
    {mode === "region"
      ? "드래그하여 영역 선택 · 우클릭 취소"
      : mode === "window"
        ? "창에 마우스를 올리고 클릭 · 우클릭 취소"
        : "클릭하여 이 디스플레이 캡처 · 우클릭 취소"}
  </div>
</div>

<style>
  .ov {
    position: fixed;
    inset: 0;
    cursor: crosshair;
  }
  .ov.nocursor {
    cursor: none;
  }
  .dim {
    position: fixed;
    inset: 0;
    background: rgba(15, 17, 20, 0.45);
    pointer-events: none;
  }
  .vline,
  .hline {
    position: fixed;
    background: rgba(45, 163, 255, 0.8);
    pointer-events: none;
    z-index: 2;
  }
  .vline {
    top: 0;
    bottom: 0;
    width: 1px;
  }
  .hline {
    left: 0;
    right: 0;
    height: 1px;
  }
  .hole {
    position: fixed;
    border: 1.5px solid #2da3ff;
    box-shadow: 0 0 0 9999px rgba(15, 17, 20, 0.45);
    pointer-events: none;
    z-index: 1;
  }
  .badge {
    position: absolute;
    top: -26px;
    left: 0;
    background: rgba(20, 22, 26, 0.92);
    color: #fff;
    font:
      11px/1 ui-monospace,
      SFMono-Regular,
      monospace;
    padding: 4px 7px;
    border-radius: 5px;
    white-space: nowrap;
    max-width: 80vw;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* window 호버 대상: 살짝 파란 틴트 + 테두리 + 가운데 카메라 */
  .target {
    position: fixed;
    border: 2px solid #2da3ff;
    border-radius: 4px;
    background: rgba(45, 120, 220, 0.18);
    pointer-events: none;
    z-index: 2;
  }
  .cam {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.6));
  }
  .dcenter {
    position: fixed;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    pointer-events: none;
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.6));
  }
  .displaylabel {
    background: rgba(20, 22, 26, 0.88);
    color: #fff;
    padding: 10px 22px;
    border-radius: 999px;
    font-size: 15px;
    font-weight: 600;
  }
  .loupe {
    position: fixed;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.9);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.45);
    image-rendering: pixelated;
    pointer-events: none;
    overflow: hidden;
    background-repeat: no-repeat;
    z-index: 3;
  }
  .loupe-x,
  .loupe-y {
    position: absolute;
    background: rgba(45, 163, 255, 0.85);
  }
  .loupe-x {
    left: 0;
    right: 0;
    top: 50%;
    height: 1px;
  }
  .loupe-y {
    top: 0;
    bottom: 0;
    left: 50%;
    width: 1px;
  }
  .loupe-cell {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 8px;
    height: 8px;
    transform: translate(-50%, -50%);
    border: 1px solid rgba(255, 255, 255, 0.9);
  }
  .loupe-coord {
    position: absolute;
    bottom: 6px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(20, 22, 26, 0.9);
    color: #fff;
    font:
      10px/1 ui-monospace,
      monospace;
    padding: 2px 6px;
    border-radius: 4px;
    white-space: nowrap;
  }
  .hint {
    position: fixed;
    top: 20px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(20, 22, 26, 0.82);
    color: #e8eaed;
    padding: 8px 18px;
    border-radius: 999px;
    font-size: 13px;
    pointer-events: none;
    z-index: 4;
  }
</style>
