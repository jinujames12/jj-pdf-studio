<script lang="ts">
  import TitleBar from './lib/components/TitleBar.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Home from './lib/components/Home.svelte';
  import About from './lib/components/About.svelte';
  import ToolPlaceholder from './lib/components/ToolPlaceholder.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import Pages from './lib/tools/Pages.svelte';
  import { nav } from './lib/stores/nav.svelte';
  import { tools } from './lib/tools';
  import { onMount } from 'svelte';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { inTauri } from './lib/api';
  import { documents, openFiles } from './lib/stores/documents.svelte';

  const tool = $derived(tools.find((t) => t.id === nav.view));

  // Files dropped anywhere on the window open in the Pages tool. Tauri reports
  // drops through the webview (HTML5 drop events do not fire) with real paths.
  onMount(() => {
    if (!inTauri()) return;
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      const p = event.payload;
      if (p.type === 'enter' || p.type === 'over') documents.dragging = true;
      else if (p.type === 'leave') documents.dragging = false;
      else if (p.type === 'drop') {
        documents.dragging = false;
        void openFiles(p.paths);
      }
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  });
</script>

<div class="shell">
  <TitleBar />
  <div class="body">
    <Sidebar />
    <main class="content">
      {#key nav.view}
        <div class="view">
          {#if nav.view === 'home'}
            <Home />
          {:else if nav.view === 'about'}
            <About />
          {:else if nav.view === 'pages'}
            <Pages />
          {:else if tool}
            <ToolPlaceholder {tool} />
          {/if}
        </div>
      {/key}
    </main>
  </div>
  <Toasts />
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .content {
    flex: 1;
    min-width: 0;
    overflow: auto;
  }
  .view {
    min-height: 100%;
    animation: view-in var(--duration-base) var(--ease);
  }
  @keyframes view-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
</style>
