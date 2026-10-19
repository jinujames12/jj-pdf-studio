// Previews for the image list. Rust makes small JPEG/PNG thumbnails; this keeps
// at most a few requests in flight and turns the bytes into blob URLs.
import { imageThumbnail } from './api';

const THUMB_EDGE = 320;
const MAX_CONCURRENT = 2;

let running = 0;
const waiting: Array<() => void> = [];
const urls = new Map<string, Promise<string>>();

async function slot<T>(work: () => Promise<T>): Promise<T> {
  if (running >= MAX_CONCURRENT) await new Promise<void>((resolve) => waiting.push(resolve));
  running++;
  try {
    return await work();
  } finally {
    running--;
    waiting.shift()?.();
  }
}

/** Blob URL of the preview for `path` (created once per path). */
export function thumbUrl(path: string): Promise<string> {
  let url = urls.get(path);
  if (!url) {
    url = slot(async () => {
      const bytes = await imageThumbnail(path, THUMB_EDGE);
      // PNG signature; everything else the backend sends is JPEG.
      const type = bytes[0] === 0x89 ? 'image/png' : 'image/jpeg';
      return URL.createObjectURL(new Blob([bytes as BlobPart], { type }));
    });
    // A failed load can be retried the next time the card asks.
    url.catch(() => urls.delete(path));
    urls.set(path, url);
  }
  return url;
}

export function forgetThumb(path: string) {
  const url = urls.get(path);
  urls.delete(path);
  void url?.then((u) => URL.revokeObjectURL(u)).catch(() => {});
}

export function clearThumbs() {
  for (const path of [...urls.keys()]) forgetThumb(path);
}
