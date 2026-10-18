<script lang="ts">
  import TitleBar from './lib/components/TitleBar.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Home from './lib/components/Home.svelte';
  import About from './lib/components/About.svelte';
  import ToolPlaceholder from './lib/components/ToolPlaceholder.svelte';
  import { nav } from './lib/stores/nav.svelte';
  import { tools } from './lib/tools';

  const tool = $derived(tools.find((t) => t.id === nav.view));
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
          {:else if tool}
            <ToolPlaceholder {tool} />
          {/if}
        </div>
      {/key}
    </main>
  </div>
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
