// Drawing one page as a PNG or JPEG picture for PDF to Images.
import type { PdfDoc } from './render';

/** Chromium (WebView2) cannot make a canvas larger than this on a side... */
const MAX_SIDE = 16384;
/** ...and a canvas this big already needs 400 MB, so stop earlier. */
const MAX_PIXELS = 100_000_000;

export type PictureFormat = 'png' | 'jpg';

/** Pixel size of a page of `width` x `height` points at `dpi`. */
export function pixelSize(width: number, height: number, dpi: number) {
  const k = dpi / 72;
  return { width: Math.max(1, Math.round(width * k)), height: Math.max(1, Math.round(height * k)) };
}

/**
 * Message for the first page that would be too large at `dpi`, or `null`.
 * `sizes` are on-screen page sizes in points, first page first.
 */
export function tooLarge(
  sizes: Array<{ page: number; width: number; height: number }>,
  dpi: number
): string | null {
  for (const s of sizes) {
    const px = pixelSize(s.width, s.height, dpi);
    if (px.width > MAX_SIDE || px.height > MAX_SIDE || px.width * px.height > MAX_PIXELS) {
      const longest = Math.max(s.width, s.height);
      const area = s.width * s.height;
      const safe = Math.floor(
        Math.min((MAX_SIDE / longest) * 72, Math.sqrt(MAX_PIXELS / area) * 72)
      );
      return `Page ${s.page} would be ${px.width} × ${px.height} pixels, which is too large to draw. Use ${safe} DPI or less.`;
    }
  }
  return null;
}

/**
 * Renders page `pageNumber` at `dpi` and encodes it. The page keeps its own
 * rotation and size, so rotated and mixed-size documents come out right.
 * The background is white because JPEG has no transparency.
 */
export async function renderPicture(
  pdf: PdfDoc,
  pageNumber: number,
  dpi: number,
  format: PictureFormat,
  quality: number
): Promise<Uint8Array> {
  const page = await pdf.getPage(pageNumber);
  const canvas = document.createElement('canvas');
  try {
    const viewport = page.getViewport({ scale: dpi / 72 });
    canvas.width = Math.round(viewport.width);
    canvas.height = Math.round(viewport.height);
    await page.render({ canvas, viewport, background: 'white' }).promise;
    const blob = await new Promise<Blob | null>((resolve) =>
      canvas.toBlob(
        resolve,
        format === 'png' ? 'image/png' : 'image/jpeg',
        format === 'jpg' ? quality / 100 : undefined
      )
    );
    if (!blob) throw new Error(`Page ${pageNumber} could not be turned into a picture.`);
    return new Uint8Array(await blob.arrayBuffer());
  } finally {
    // Give the memory back now; the next page may be just as large.
    canvas.width = canvas.height = 0;
    page.cleanup();
  }
}
