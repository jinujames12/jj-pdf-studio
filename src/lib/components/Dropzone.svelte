<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { open } from '@tauri-apps/plugin-dialog';
  import FileUp from 'lucide-svelte/icons/file-up';
  import { inTauri } from '../api';

  // M0 only proves the plumbing: we accept paths from the dialog / drop and
  // say how many arrived. Actually opening PDFs is Milestone M1.
  let hovering = $state(false);
  let notice = $state('');

  function accept(paths: string[]) {
    const n = paths.length;
    notice = `${n} file${n === 1 ? '' : 's'} received. Opening PDFs arrives in the next milestone.`;
  }

  async function browse() {
    const picked = await open({
      multiple: true,
      filters: [{ name: 'PDF and images', extensions: ['pdf', 'jpg', 'jpeg', 'png'] }]
    });
    if (picked) accept(picked);
  }

  // With Tauri's native drag-drop enabled, the webview reports drops through
  // its own event (HTML5 drop events do not fire), and gives real file paths.
  onMount(() => {
    if (!inTauri()) return;
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      const p = event.payload;
      if (p.type === 'enter' || p.type === 'over') hovering = true;
      else if (p.type === 'leave') hovering = false;
      else if (p.type === 'drop') {
        hovering = false;
        accept(p.paths);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  });
</script>

<button class="dropzone" class:hovering onclick={browse}>
  <span class="badge"><FileUp size={28} strokeWidth={1.75} /></span>
  <span class="title">Drop PDFs here or click to browse</span>
  <span class="hint">Your files stay on this computer. Originals are never changed.</span>
  {#if notice}<span class="notice" role="status">{notice}</span>{/if}
</button>

<style>
  .dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    width: 100%;
    min-height: 200px;
    padding: var(--space-8) var(--space-6);
    border: 2px dashed var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    cursor: pointer;
    transition:
      border-color var(--duration-base) var(--ease),
      background var(--duration-base) var(--ease);
  }
  .dropzone:hover,
  .dropzone.hovering {
    border-color: var(--primary);
    background: var(--selected-bg);
  }
  .badge {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: var(--radius-card);
    background: var(--selected-bg);
    color: var(--primary);
    transition: transform var(--duration-base) var(--ease);
  }
  .hovering .badge {
    transform: translateY(-2px);
  }
  .title {
    font-size: var(--text-header);
    font-weight: 600;
  }
  .hint {
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
  .notice {
    margin-top: var(--space-2);
    color: var(--primary);
    font-size: var(--text-caption);
    font-weight: 500;
  }
</style>
