// Pure helpers for the Page Editor panels, kept free of Svelte so they can be tested.
import type { PageNumberOptions } from '../api';

export type SplitKind = 'every' | 'ranges' | 'at';

export interface SplitView {
  kind: SplitKind;
  /** Pages per file (Every N). */
  perFile: number;
  /** Inclusive 1-based ranges from the typed text (By ranges). */
  ranges: ReadonlyArray<[number, number]>;
  /** Pages after which the document is cut (At markers), 1-based grid positions. */
  cuts: ReadonlySet<number>;
}

/** Items to split into files for a mode, ready to label on the grid. */
export function splitFileCount(view: SplitView, pageCount: number): number {
  if (view.kind === 'every') return Math.ceil(pageCount / view.perFile);
  if (view.kind === 'ranges') return view.ranges.length;
  return view.cuts.size + 1;
}

/** True when the mode has enough input to run. */
export function splitReady(view: SplitView, pageCount: number): boolean {
  if (view.kind === 'every') return pageCount > 0;
  if (view.kind === 'ranges') return view.ranges.length > 0;
  return view.cuts.size > 0;
}

/** Which output file a page lands in (-1: none). */
export function splitGroupOf(view: SplitView, page: number): number {
  if (view.kind === 'every') return Math.floor((page - 1) / view.perFile);
  if (view.kind === 'ranges') return view.ranges.findIndex(([a, b]) => page >= a && page <= b);
  let g = 0;
  for (const m of view.cuts) if (m < page) g++;
  return g;
}

/** The alternating tint of a page, so neighbouring parts look different. */
export function splitTint(view: SplitView, page: number): 'a' | 'b' | null {
  const g = splitGroupOf(view, page);
  return g < 0 ? null : g % 2 === 0 ? 'a' : 'b';
}

/** "Part 3" on the first page of each part, nothing on the others. */
export function splitPartLabel(view: SplitView, page: number): string | null {
  const g = splitGroupOf(view, page);
  if (g < 0) return null;
  return page === 1 || splitGroupOf(view, page - 1) !== g ? `Part ${g + 1}` : null;
}

/**
 * Turns markers kept as item ids ("cut after this page", stable while pages are
 * moved) into the grid positions Rust needs. The last page cannot be cut after.
 */
export function markerPositions(markers: ReadonlySet<number>, ids: readonly number[]): number[] {
  return ids.flatMap((id, k) => (markers.has(id) && k < ids.length - 1 ? [k + 1] : []));
}

/** Expands inclusive ranges to page numbers. */
export function rangesToPages(ranges: ReadonlyArray<[number, number]>): number[] {
  return ranges.flatMap(([a, b]) => Array.from({ length: b - a + 1 }, (_, i) => a + i));
}

/**
 * The label drawn on each numbered page (the same rule as `page_number_labels` in
 * pdf-core). `numbered` is the 1-based positions that get a number, or `null` for all.
 * Returns position -> text.
 */
export function numberLabels(
  options: Pick<PageNumberOptions, 'format' | 'start'>,
  pageCount: number,
  numbered: readonly number[] | null
): Map<number, string> {
  const chosen = numbered ?? Array.from({ length: pageCount }, (_, i) => i + 1);
  const last = options.start + chosen.length - 1;
  return new Map(
    chosen.map((position, i) => [
      position,
      options.format
        .replaceAll('{n}', String(options.start + i))
        .replaceAll('{total}', String(last))
    ])
  );
}

export const NUMBER_FORMATS: Array<[string, string]> = [
  ['{n}', '1'],
  ['Page {n}', 'Page 1'],
  ['Page {n} of {total}', 'Page 1 of N'],
  ['{n} / {total}', '1 / N']
];

/** Default name of a saved edit, next to the source: `name (edited).pdf`. */
export const editedName = (stem: string) => `${stem} (edited).pdf`;
