// pdf.js wrapper: loads documents and renders thumbnails into a small cache.
import * as pdfjs from 'pdfjs-dist';
import type { PDFDocumentProxy, RenderTask } from 'pdfjs-dist';
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';

// The worker ships inside the app bundle, so no network is involved.
pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export type PdfDoc = PDFDocumentProxy;

export function loadPdf(bytes: Uint8Array): Promise<PdfDoc> {
  return pdfjs.getDocument({ data: bytes }).promise;
}

/** Thumbnail width in CSS pixels (SPEC section 3). */
export const THUMB_WIDTH = 160;
const MAX_CACHED = 300;

// Only a few pages render at once, and a queued page whose cell has scrolled
// away is dropped before it starts, so fast scrolling through a long document
// never builds up a backlog.
const MAX_CONCURRENT = 3;
let running = 0;
const waiting: Array<() => void> = [];

async function acquireSlot(signal?: AbortSignal): Promise<void> {
  if (running < MAX_CONCURRENT) {
    running++;
    return;
  }
  await new Promise<void>((resolve, reject) => {
    const start = () => {
      signal?.removeEventListener('abort', cancel);
      running++;
      resolve();
    };
    const cancel = () => {
      const at = waiting.indexOf(start);
      if (at >= 0) waiting.splice(at, 1);
      reject(new DOMException('Render cancelled', 'AbortError'));
    };
    if (signal?.aborted) return cancel();
    signal?.addEventListener('abort', cancel, { once: true });
    waiting.push(start);
  });
}

function releaseSlot() {
  running--;
  waiting.shift()?.();
}

// Insertion-ordered Map doubles as a simple LRU: re-inserting moves to the end.
const cache = new Map<string, ImageBitmap>();
const inflight = new Map<string, Promise<ImageBitmap>>();

/**
 * Renders page `pageNumber` (1-based) at `rotation` degrees (absolute, i.e. the
 * page's own /Rotate plus any user turn). Results are cached per document.
 */
export function renderThumb(
  pdf: PdfDoc,
  docKey: string,
  pageNumber: number,
  rotation: number,
  signal?: AbortSignal
): Promise<ImageBitmap> {
  const key = `${docKey}:${pageNumber}:${rotation}`;
  const hit = cache.get(key);
  if (hit) {
    cache.delete(key);
    cache.set(key, hit);
    return Promise.resolve(hit);
  }
  const pending = inflight.get(key);
  if (pending) return pending;

  const job = (async () => {
    await acquireSlot(signal);
    try {
      return await draw();
    } finally {
      releaseSlot();
    }
  })().finally(() => inflight.delete(key));
  inflight.set(key, job);
  return job;

  async function draw(): Promise<ImageBitmap> {
    const page = await pdf.getPage(pageNumber);
    const base = page.getViewport({ scale: 1, rotation });
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const viewport = page.getViewport({ scale: (THUMB_WIDTH * dpr) / base.width, rotation });
    const canvas = document.createElement('canvas');
    canvas.width = Math.ceil(viewport.width);
    canvas.height = Math.ceil(viewport.height);
    await page.render({ canvas, viewport, background: 'white' }).promise;
    page.cleanup();
    const bitmap = await createImageBitmap(canvas);
    cache.set(key, bitmap);
    while (cache.size > MAX_CACHED) {
      const oldest = cache.keys().next().value!;
      cache.get(oldest)?.close();
      cache.delete(oldest);
    }
    return bitmap;
  }
}

export interface PreviewHandle {
  promise: Promise<void>;
  cancel: () => void;
}

/**
 * Draws one page large enough to fill `boxWidth` x `boxHeight` CSS pixels
 * (without enlarging past the box) onto `canvas`. Call `cancel` to abandon it.
 */
export function renderPreview(
  pdf: PdfDoc,
  canvas: HTMLCanvasElement,
  pageNumber: number,
  rotation: number,
  boxWidth: number,
  boxHeight: number
): PreviewHandle {
  let cancelled = false;
  let task: RenderTask | undefined;
  const promise = (async () => {
    const page = await pdf.getPage(pageNumber);
    if (cancelled) return;
    const base = page.getViewport({ scale: 1, rotation });
    const fit = Math.min(boxWidth / base.width, boxHeight / base.height);
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const viewport = page.getViewport({ scale: fit * dpr, rotation });
    canvas.width = Math.ceil(viewport.width);
    canvas.height = Math.ceil(viewport.height);
    canvas.style.width = `${Math.floor(base.width * fit)}px`;
    canvas.style.height = `${Math.floor(base.height * fit)}px`;
    task = page.render({ canvas, viewport, background: 'white' });
    await task.promise;
    page.cleanup();
  })();
  return {
    promise,
    cancel() {
      cancelled = true;
      task?.cancel();
    }
  };
}

/** Drops every cached bitmap of one document (call when it is closed). */
export function clearThumbs(docKey: string) {
  for (const [key, bmp] of cache) {
    if (key.startsWith(`${docKey}:`)) {
      bmp.close();
      cache.delete(key);
    }
  }
}
