<script lang="ts">
  // 같은 번들이 메인 창과 (모니터별) 오버레이 창에서 로드된다. 창 label로 UI를 분기하고,
  // 창 종류에 맞게 body 스타일을 직접 지정한다(전역 CSS가 서로 새지 않도록).
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Main from "$lib/Main.svelte";
  import Overlay from "$lib/Overlay.svelte";

  let label = $state<string | null>(null);
  let isOverlay = $state(false);

  onMount(() => {
    let l = "main";
    try {
      l = getCurrentWindow().label;
    } catch {
      l = "main";
    }
    label = l;
    isOverlay = l.startsWith("overlay");
    if (isOverlay) {
      document.documentElement.style.background = "transparent";
      document.body.style.background = "transparent";
      document.body.style.overflow = "hidden";
      document.body.style.margin = "0";
    } else {
      document.body.style.overflow = "auto";
    }
  });
</script>

{#if label === null}
  <!-- 초기화 대기 -->
{:else if isOverlay}
  <Overlay />
{:else}
  <Main />
{/if}
