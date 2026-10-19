// Long-running Rust jobs: tracks their progress events and lets the user cancel them.
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface Job {
  id: string;
  label: string;
  done: number;
  total: number;
  message: string;
  cancelling: boolean;
}

/** The error text pdf-core uses when a job was cancelled. */
export const CANCELLED = 'Cancelled.';

export const jobs = $state<{ list: Job[] }>({ list: [] });

let nextId = 1;
let listening: Promise<unknown> | null = null;

/** Starts listening for `progress` events (once). */
function ensureListener() {
  if (!listening && isTauri()) {
    listening = listen<{ jobId: string; done: number; total: number; message: string }>(
      'progress',
      (event) => {
        const p = event.payload;
        const job = jobs.list.find((j) => j.id === p.jobId);
        if (job) {
          job.done = p.done;
          job.total = p.total;
          job.message = p.message;
        }
      }
    );
  }
  return listening ?? Promise.resolve();
}

/**
 * Runs `start` with a fresh job id and shows a progress bar with a Cancel button until it
 * settles. A cancelled job rejects with `CANCELLED`.
 */
export async function runJob<T>(label: string, start: (jobId: string) => Promise<T>): Promise<T> {
  await ensureListener();
  const id = `job-${nextId++}`;
  jobs.list.push({ id, label, done: 0, total: 0, message: '', cancelling: false });
  try {
    return await start(id);
  } finally {
    jobs.list = jobs.list.filter((j) => j.id !== id);
  }
}

export function cancelJob(id: string) {
  const job = jobs.list.find((j) => j.id === id);
  if (!job || job.cancelling) return;
  job.cancelling = true;
  void invoke('cancel_job', { jobId: id });
}
