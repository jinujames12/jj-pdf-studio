<script lang="ts">
  import FileUp from 'lucide-svelte/icons/file-up';
  import { browseForPdf, documents } from '../stores/documents.svelte';
</script>

<button class="dropzone" class:hovering={documents.dragging} onclick={browseForPdf}>
  <span class="badge"><FileUp size={28} strokeWidth={1.75} /></span>
  <span class="title">Drop PDFs here or click to browse</span>
  <span class="hint">Your files stay on this computer. Originals are never changed.</span>
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
</style>
