<script lang="ts">
  import TitleBar from './lib/components/TitleBar.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Home from './lib/components/Home.svelte';
  import About from './lib/components/About.svelte';
  import ToolPlaceholder from './lib/components/ToolPlaceholder.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import JobProgress from './lib/components/JobProgress.svelte';
  import Shortcuts from './lib/components/Shortcuts.svelte';
  import Notices from './lib/components/Notices.svelte';
  import PasswordPrompt from './lib/components/PasswordPrompt.svelte';
  import Pages from './lib/tools/Pages.svelte';
  import Merge from './lib/tools/Merge.svelte';
  import Split from './lib/tools/Split.svelte';
  import Extract from './lib/tools/Extract.svelte';
  import Rotate from './lib/tools/Rotate.svelte';
  import Resize from './lib/tools/Resize.svelte';
  import Compress from './lib/tools/Compress.svelte';
  import Images from './lib/tools/Images.svelte';
  import Unlock from './lib/tools/Unlock.svelte';
  import Protect from './lib/tools/Protect.svelte';
  import ToImages from './lib/tools/ToImages.svelte';
  import PageNumbers from './lib/tools/PageNumbers.svelte';
  import Watermark from './lib/tools/Watermark.svelte';
  import Properties from './lib/tools/Properties.svelte';
  import { nav } from './lib/stores/nav.svelte';
  import { tools } from './lib/tools';
  import { onMount } from 'svelte';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { inTauri } from './lib/api';
  import { browseForPdf, documents, openFiles } from './lib/stores/documents.svelte';
  import { dialogs } from './lib/stores/dialogs.svelte';

  let main = $state<HTMLElement>();

  const tool = $derived(tools.find((t) => t.id === nav.view));

  function onkeydown(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    const typing = !!t && /^(INPUT|SELECT|TEXTAREA)$/.test(t.tagName);
    if (e.key === 'F1') {
      e.preventDefault();
      dialogs.open = 'shortcuts';
    } else if (
      (e.ctrlKey || e.metaKey) &&
      e.key.toLowerCase() === 'o' &&
      !typing &&
      (nav.view === 'home' || nav.view === 'about')
    ) {
      // Tool screens handle Ctrl+O themselves; Home and About have no handler.
      e.preventDefault();
      void browseForPdf();
    }
  }

  // Files dropped anywhere on the window open in the Pages tool (or join the list in Merge or
  // Images to PDF). Tauri reports
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

<!-- Shortcuts that work on every screen. Tools add their own (Ctrl+S, Ctrl+Z...). -->
<svelte:window {onkeydown} />

<div class="shell">
  <button class="skip" onclick={() => main?.focus()}>Skip to content</button>
  <TitleBar />
  <div class="body">
    <Sidebar />
    <main class="content" id="main" tabindex="-1" bind:this={main}>
      {#key nav.view}
        <div class="view">
          {#if nav.view === 'home'}
            <Home />
          {:else if nav.view === 'about'}
            <About />
          {:else if nav.view === 'pages'}
            <Pages />
          {:else if nav.view === 'merge'}
            <Merge />
          {:else if nav.view === 'split'}
            <Split />
          {:else if nav.view === 'extract'}
            <Extract />
          {:else if nav.view === 'rotate'}
            <Rotate />
          {:else if nav.view === 'resize'}
            <Resize />
          {:else if nav.view === 'compress'}
            <Compress />
          {:else if nav.view === 'images'}
            <Images />
          {:else if nav.view === 'unlock'}
            <Unlock />
          {:else if nav.view === 'protect'}
            <Protect />
          {:else if nav.view === 'toimages'}
            <ToImages />
          {:else if nav.view === 'pagenumbers'}
            <PageNumbers />
          {:else if nav.view === 'watermark'}
            <Watermark />
          {:else if nav.view === 'properties'}
            <Properties />
          {:else if tool}
            <ToolPlaceholder {tool} />
          {/if}
        </div>
      {/key}
    </main>
  </div>
  <Toasts />
  <JobProgress />
  {#if dialogs.open === 'shortcuts'}<Shortcuts />{/if}
  {#if dialogs.open === 'notices'}<Notices />{/if}
  <PasswordPrompt />
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  /* Hidden until a keyboard user tabs to it. */
  .skip {
    position: absolute;
    left: var(--space-2);
    top: -48px;
    z-index: 80;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface);
    cursor: pointer;
  }
  .skip:focus {
    top: var(--space-2);
  }
  .content:focus {
    outline: none;
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
