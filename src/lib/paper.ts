// Page-size choices shared by Images to PDF and Resize.
import type { Paper } from './api';

export const MM_PER_INCH = 25.4;
const PT_PER_MM = 72 / MM_PER_INCH;

export type PaperKind = 'fit' | 'a4' | 'a3' | 'a5' | 'letter' | 'legal' | 'custom';

/** What the page-size controls hold. */
export interface PaperChoice {
  kind: PaperKind;
  /** Custom size, in `unit`. */
  width: number;
  height: number;
  unit: 'mm' | 'in';
}

export const defaultPaper = (kind: PaperKind = 'a4'): PaperChoice => ({
  kind,
  width: 210,
  height: 297,
  unit: 'mm'
});

export const PRESET_LABELS: Record<Exclude<PaperKind, 'fit' | 'custom'>, string> = {
  a4: 'A4 (210 × 297 mm)',
  a3: 'A3 (297 × 420 mm)',
  a5: 'A5 (148 × 210 mm)',
  letter: 'Letter (8.5 × 11 in)',
  legal: 'Legal (8.5 × 14 in)'
};

const toMm = (v: number, unit: 'mm' | 'in') => (unit === 'in' ? v * MM_PER_INCH : v);

/** True if a custom size is usable (the Rust side checks again). */
export function validCustom(c: PaperChoice): boolean {
  const w = toMm(c.width, c.unit);
  const h = toMm(c.height, c.unit);
  return Number.isFinite(w) && Number.isFinite(h) && w >= 10 && h >= 10 && w <= 5000 && h <= 5000;
}

/** The size to send to Rust. `null` means "fit to image". */
export function toPaper(c: PaperChoice): Paper | null {
  if (c.kind === 'fit') return null;
  if (c.kind === 'custom')
    return { kind: 'custom', width_mm: toMm(c.width, c.unit), height_mm: toMm(c.height, c.unit) };
  return { kind: c.kind };
}

/** Points to a short human label, e.g. `210 × 297 mm`. */
export function fmtPoints(w: number, h: number): string {
  const mm = (v: number) => Math.round(v / PT_PER_MM);
  return `${mm(w)} × ${mm(h)} mm`;
}

/** Named size if the points match a preset (within 1 mm, either orientation). */
export function nameOfSize(w: number, h: number): string | null {
  const sizes: Array<[string, number, number]> = [
    ['A3', 297, 420],
    ['A4', 210, 297],
    ['A5', 148, 210],
    ['Letter', 215.9, 279.4],
    ['Legal', 215.9, 355.6]
  ];
  const [a, b] = [w / PT_PER_MM, h / PT_PER_MM].sort((x, y) => x - y);
  const hit = sizes.find(([, sw, sh]) => Math.abs(a - sw) < 1 && Math.abs(b - sh) < 1);
  return hit ? hit[0] : null;
}

/** `A4` when the size matches a preset, otherwise `210×297`. */
export function shortSize(w: number, h: number): string {
  const name = nameOfSize(w, h);
  if (name) return name;
  const mm = (v: number) => Math.round(v / PT_PER_MM);
  return `${mm(w)}×${mm(h)}`;
}
