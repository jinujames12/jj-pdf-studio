<script lang="ts">
  import { cancelJob, jobs } from '../stores/jobs.svelte';

  // Escape cancels the newest job, like closing a dialog.
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && jobs.list.length) cancelJob(jobs.list[jobs.list.length - 1].id);
  }
</script>

<svelte:window {onkeydown} />

{#if jobs.list.length}
  <div class="stack">
    {#each jobs.list as job (job.id)}
      <div class="job" role="status" aria-live="polite">
        <div class="top">
          <strong>{job.label}</strong>
          <button class="btn" disabled={job.cancelling} onclick={() => cancelJob(job.id)}>
            {job.cancelling ? 'Cancelling…' : 'Cancel'}
          </button>
        </div>
        <!-- No `value` while the total is unknown: the bar shows as indeterminate. -->
        <progress
          max={job.total || undefined}
          value={job.total ? job.done : undefined}
          aria-label={job.label}
        ></progress>
        <span class="msg">{job.message || 'Starting…'}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .stack {
    position: fixed;
    left: 50%;
    bottom: var(--space-4);
    transform: translateX(-50%);
    z-index: 60;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    width: min(440px, calc(100vw - 32px));
  }
  .job {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
    box-shadow: var(--shadow-card-hover);
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }
  progress {
    width: 100%;
    height: 8px;
    accent-color: var(--primary);
  }
  .msg {
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
</style>
