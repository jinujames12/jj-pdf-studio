<script lang="ts">
  import { onMount } from 'svelte';
  import { save } from '@tauri-apps/plugin-dialog';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import Lock from 'lucide-svelte/icons/lock';
  import TriangleAlert from 'lucide-svelte/icons/triangle-alert';
  import PageThumb from '../components/PageThumb.svelte';
  import Preview from '../components/Preview.svelte';
  import ToolFrame from '../components/ToolFrame.svelte';
  import { protectPdf } from '../api';
  import { plural } from '../format';
  import { folderOf, stemOf } from '../paths';
  import { announceSaved, report } from '../saved';
  import { browseForPdf, documents } from '../stores/documents.svelte';
  import type { PageItem } from '../stores/pageEdit.svelte';
  import { preview } from '../stores/preview.svelte';

  const doc = $derived(documents.current);
  const count = $derived(doc?.info.pages.length ?? 0);

  let password = $state('');
  let confirmation = $state('');
  let restrict = $state(false);
  let allowPrinting = $state(true);
  let allowCopying = $state(true);
  let allowEditing = $state(true);
  let ownerPassword = $state('');
  let working = $state(false);

  const items = $derived<PageItem[]>(
    Array.from({ length: count }, (_, i) => ({ id: i + 1, kind: 'page', src: i + 1, rotate: 0 }))
  );

  /** A rough hint, not a promise: length matters most, then a mix of characters. */
  const strength = $derived.by(() => {
    if (!password) return null;
    const kinds = [/[a-z]/, /[A-Z]/, /\d/, /[^A-Za-z0-9]/].filter((r) => r.test(password)).length;
    if (password.length >= 12 && kinds >= 3) return { label: 'Strong', level: 'strong' };
    if (password.length >= 8 && kinds >= 2) return { label: 'Fair', level: 'fair' };
    return {
      label: 'Weak: use 12 or more characters with a mix of letters and digits',
      level: 'weak'
    };
  });

  const restricting = $derived(restrict && !(allowPrinting && allowCopying && allowEditing));
  const mismatch = $derived(confirmation !== '' && confirmation !== password);
  const ownerProblem = $derived(
    restricting && ownerPassword !== '' && ownerPassword === password
      ? 'The owner password must be different from the open password.'
      : ''
  );
  const ready = $derived(
    password !== '' &&
      password === confirmation &&
      (!restricting || (ownerPassword !== '' && !ownerProblem))
  );

  // A newly opened document starts a fresh form.
  $effect(() => {
    void doc?.key;
    password = confirmation = ownerPassword = '';
    restrict = false;
    allowPrinting = allowCopying = allowEditing = true;
  });

  async function protect() {
    if (!doc || working || !ready) return;
    const output = await save({
      title: 'Save protected PDF',
      defaultPath: `${folderOf(doc.path)}${stemOf(doc.path)} (protected).pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }]
    });
    if (!output) return;
    working = true;
    try {
      announceSaved(
        await protectPdf(doc.path, output, {
          open_password: password,
          owner_password: restricting ? ownerPassword : null,
          allow_printing: allowPrinting,
          allow_copying: allowCopying,
          allow_editing: allowEditing
        })
      );
    } catch (e) {
      report(e);
    } finally {
      working = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (preview.id !== null) return;
    if (!(e.ctrlKey || e.metaKey)) return;
    const key = e.key.toLowerCase();
    if (key === 'o') {
      e.preventDefault();
      void browseForPdf();
    } else if (key === 's' && doc) {
      e.preventDefault();
      void protect();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onkeydown);
    return () => {
      window.removeEventListener('keydown', onkeydown);
      preview.close();
    };
  });
</script>

<ToolFrame
  title="Protect"
  subtitle={doc ? doc.name : 'Lock a PDF with a password (AES-256).'}
  empty={!doc}
  emptyHint="Your original is never changed. The protected copy is saved as a new file."
  onopen={browseForPdf}
>
  {#snippet actions()}
    <button class="btn" title="Open (Ctrl+O)" onclick={browseForPdf}>
      <FolderOpen size={16} strokeWidth={1.75} /> Open
    </button>
    {#if doc}
      <button
        class="btn primary"
        title="Protect (Ctrl+S)"
        disabled={working || !ready}
        onclick={protect}
      >
        <Lock size={16} strokeWidth={1.75} />
        {working ? 'Protecting…' : 'Protect…'}
      </button>
    {/if}
  {/snippet}

  <form
    class="panel"
    onsubmit={(e) => {
      e.preventDefault();
      void protect();
    }}
  >
    <label class="field">
      <span>Open password</span>
      <input type="password" bind:value={password} autocomplete="new-password" spellcheck="false" />
    </label>
    {#if strength}<p class="hint {strength.level}">{strength.label}</p>{/if}
    <label class="field">
      <span>Repeat password</span>
      <input
        type="password"
        bind:value={confirmation}
        class:invalid={mismatch}
        autocomplete="new-password"
        spellcheck="false"
      />
    </label>
    {#if mismatch}<p class="field-error">The two passwords do not match.</p>{/if}

    <label class="check">
      <input type="checkbox" bind:checked={restrict} />
      Also restrict what people can do with the file
    </label>
    {#if restrict}
      <div class="restrictions">
        <label class="check">
          <input type="checkbox" bind:checked={allowPrinting} /> Allow printing
        </label>
        <label class="check">
          <input type="checkbox" bind:checked={allowCopying} /> Allow copying text and images
        </label>
        <label class="check">
          <input type="checkbox" bind:checked={allowEditing} /> Allow editing
        </label>
        {#if restricting}
          <label class="field">
            <span>Owner password</span>
            <input
              type="password"
              bind:value={ownerPassword}
              class:invalid={ownerProblem}
              autocomplete="new-password"
              spellcheck="false"
            />
          </label>
          <p class="hint">
            Needed to change these restrictions later. Keep it different from the open password.
          </p>
          {#if ownerProblem}<p class="field-error">{ownerProblem}</p>{/if}
        {/if}
      </div>
    {/if}

    <p class="warning">
      <TriangleAlert size={16} strokeWidth={1.75} />
      <span>
        If you forget the password, this tool cannot recover it and the file cannot be opened. The
        file uses AES-256 encryption. Restrictions are only honoured by well-behaved PDF viewers.
      </span>
    </p>
  </form>

  <div class="page-grid" role="list" aria-label="Pages">
    {#each items as item (item.id)}
      <PageThumb
        {item}
        position={item.src}
        pdf={doc!.pdf}
        docKey={doc!.key}
        readonly
        label={`${item.src}`}
        onpreview={() => preview.open(item.id)}
      />
    {/each}
  </div>

  {#if doc && preview.id !== null}
    <Preview
      pdf={doc.pdf}
      {items}
      id={preview.id}
      onnavigate={(id) => preview.open(id)}
      onclose={() => preview.close()}
    />
  {/if}

  {#snippet footer()}
    <span>{plural(count, 'page')}</span>
    <span>The protected copy is saved as a new file.</span>
  {/snippet}
</ToolFrame>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    max-width: 440px;
    margin: var(--space-4);
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .field input {
    height: 32px;
    padding: 0 var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
  }
  .field input.invalid {
    border-color: var(--danger);
  }
  .check {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  .restrictions {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-left: var(--space-6);
  }
  p {
    margin: 0;
  }
  .hint {
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
  .hint.weak {
    color: var(--danger);
  }
  .hint.fair {
    color: var(--warning);
  }
  .hint.strong {
    color: var(--success);
  }
  .warning {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-2);
    padding: var(--space-3);
    border-radius: var(--radius-control);
    background: var(--selected-bg);
    color: var(--text-muted);
    font-size: var(--text-caption);
  }
  .warning :global(svg) {
    flex: none;
    color: var(--warning);
  }
</style>
