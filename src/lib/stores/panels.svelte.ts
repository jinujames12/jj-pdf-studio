// Settings of the Page Editor panels. They live here, not in the panel components,
// so closing a panel keeps its settings for the session (SPEC section 5.15).
// The Sets are only ever replaced, never mutated.
/* eslint-disable svelte/prefer-svelte-reactivity */
import type { PageNumberOptions, RotateTarget } from '../api';
import { markerPositions, type SplitKind, type SplitView } from '../pageEditor/logic';

export const splitSettings = $state({
  mode: 'every' as SplitKind,
  every: 1 as number | null,
  rangesText: '',
  /** Ranges parsed by Rust from `rangesText` (live validation). */
  parsed: [] as Array<[number, number]>,
  rangeError: '',
  /** Ids of the pages the document is cut after. */
  markers: new Set<number>() as ReadonlySet<number>
});

export const extractSettings = $state({
  output: 'one' as 'one' | 'each',
  text: '',
  textError: ''
});

export const rotateSettings = $state({
  target: 'all' as 'all' | 'odd' | 'even' | 'pages',
  pagesText: ''
});

export const numberSettings = $state({
  position: 'bottom-center' as PageNumberOptions['position'],
  margin: 10,
  fontSize: 11,
  color: '#000000',
  preset: 'Page {n} of {total}',
  custom: 'Page {n}',
  start: 1,
  range: 'all' as 'all' | 'pages',
  pagesText: ''
});

/** The split settings as the grid and the Split button read them; `ids` are the page ids in grid order. */
export function splitView(ids: readonly number[]): SplitView {
  return {
    kind: splitSettings.mode,
    perFile: Math.max(1, Math.floor(splitSettings.every ?? 1)),
    ranges: splitSettings.parsed,
    cuts: new Set(markerPositions(splitSettings.markers, ids))
  };
}

/** The rotate target for Rust. */
export function rotateTarget(): RotateTarget {
  return rotateSettings.target === 'pages'
    ? { kind: 'pages', text: rotateSettings.pagesText }
    : { kind: rotateSettings.target };
}

/** Page numbers as currently set in the form, or `null` while the form is incomplete. */
export function numberOptions(): PageNumberOptions | null {
  const s = numberSettings;
  const format = s.preset === 'custom' ? s.custom : s.preset;
  const marginOk = Number.isFinite(s.margin) && s.margin >= 0 && s.margin <= 100;
  const sizeOk = Number.isFinite(s.fontSize) && s.fontSize >= 1 && s.fontSize <= 500;
  const startOk = Number.isInteger(s.start) && Math.abs(s.start) <= 1_000_000;
  const complete =
    marginOk &&
    sizeOk &&
    startOk &&
    format.trim() !== '' &&
    (s.range === 'all' || !!s.pagesText.trim());
  if (!complete) return null;
  return {
    position: s.position,
    margin_mm: s.margin,
    font_size: s.fontSize,
    color: s.color,
    format,
    start: s.start,
    pages: s.range === 'pages' ? s.pagesText : null
  };
}

/** A new document starts with fresh panel settings. */
export function resetPanels() {
  splitSettings.mode = 'every';
  splitSettings.every = 1;
  splitSettings.rangesText = '';
  splitSettings.parsed = [];
  splitSettings.rangeError = '';
  splitSettings.markers = new Set();
  extractSettings.output = 'one';
  extractSettings.text = '';
  extractSettings.textError = '';
  rotateSettings.target = 'all';
  rotateSettings.pagesText = '';
  numberSettings.range = 'all';
  numberSettings.pagesText = '';
}
