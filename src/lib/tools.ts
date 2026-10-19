import Merge from 'lucide-svelte/icons/merge';
import LayoutGrid from 'lucide-svelte/icons/layout-grid';
import Scissors from 'lucide-svelte/icons/scissors';
import FileOutput from 'lucide-svelte/icons/file-output';
import RotateCw from 'lucide-svelte/icons/rotate-cw';
import Scaling from 'lucide-svelte/icons/scaling';
import Minimize2 from 'lucide-svelte/icons/minimize-2';
import ImagePlus from 'lucide-svelte/icons/image-plus';
import LockOpen from 'lucide-svelte/icons/lock-open';
import Lock from 'lucide-svelte/icons/lock';
import Images from 'lucide-svelte/icons/images';
import Hash from 'lucide-svelte/icons/hash';
import Stamp from 'lucide-svelte/icons/stamp';
import FileText from 'lucide-svelte/icons/file-text';
import type { PanelId, ViewId } from './stores/nav.svelte';

export type ToolGroup = 'Organise' | 'Convert' | 'Optimise' | 'Security' | 'Edit';

/** Sidebar headings, in order (SPEC section 4). */
export const toolGroups: ToolGroup[] = ['Organise', 'Convert', 'Optimise', 'Security', 'Edit'];

export interface ToolDef {
  /** Unique key; also the old route name for the tasks that moved into the Page Editor. */
  id: string;
  /** The screen this task opens. */
  view: Exclude<ViewId, 'home' | 'about'>;
  /** The Page Editor panel that opens with it. */
  panel?: PanelId;
  /** False for tasks that have a home screen card but no sidebar entry. */
  sidebar: boolean;
  group: ToolGroup;
  name: string;
  description: string;
  icon: typeof Merge;
}

/** The PDF tools, in sidebar order. */
export const tools: ToolDef[] = [
  {
    id: 'page-editor',
    view: 'page-editor',
    sidebar: true,
    group: 'Organise',
    name: 'Page Editor',
    description: 'Reorder, rotate, delete, split, extract and number pages.',
    icon: LayoutGrid
  },
  {
    id: 'merge',
    view: 'merge',
    sidebar: true,
    group: 'Organise',
    name: 'Merge',
    description: 'Combine several PDFs into one document.',
    icon: Merge
  },
  {
    id: 'split',
    view: 'page-editor',
    panel: 'split',
    sidebar: false,
    group: 'Organise',
    name: 'Split',
    description: 'Cut a PDF into several smaller files.',
    icon: Scissors
  },
  {
    id: 'extract',
    view: 'page-editor',
    panel: 'extract',
    sidebar: false,
    group: 'Organise',
    name: 'Extract',
    description: 'Pull selected pages out into a new PDF.',
    icon: FileOutput
  },
  {
    id: 'rotate',
    view: 'page-editor',
    panel: 'rotate',
    sidebar: false,
    group: 'Organise',
    name: 'Rotate',
    description: 'Turn all or some pages, losslessly.',
    icon: RotateCw
  },
  {
    id: 'pagenumbers',
    view: 'page-editor',
    panel: 'numbers',
    sidebar: false,
    group: 'Edit',
    name: 'Page numbers',
    description: 'Add page numbers to every page.',
    icon: Hash
  },
  {
    id: 'resize',
    view: 'resize',
    sidebar: true,
    group: 'Optimise',
    name: 'Resize',
    description: 'Change page size to A4, Letter and more.',
    icon: Scaling
  },
  {
    id: 'compress',
    view: 'compress',
    sidebar: true,
    group: 'Optimise',
    name: 'Compress',
    description: 'Make a PDF smaller for email and storage.',
    icon: Minimize2
  },
  {
    id: 'images',
    view: 'images',
    sidebar: true,
    group: 'Convert',
    name: 'Images to PDF',
    description: 'Turn JPG and PNG images into a PDF.',
    icon: ImagePlus
  },
  {
    id: 'toimages',
    view: 'toimages',
    sidebar: true,
    group: 'Convert',
    name: 'PDF to Images',
    description: 'Save pages as PNG or JPG pictures.',
    icon: Images
  },
  {
    id: 'unlock',
    view: 'unlock',
    sidebar: true,
    group: 'Security',
    name: 'Unlock',
    description: 'Remove a password or restrictions from a PDF.',
    icon: LockOpen
  },
  {
    id: 'protect',
    view: 'protect',
    sidebar: true,
    group: 'Security',
    name: 'Protect',
    description: 'Lock a PDF with a password (AES-256).',
    icon: Lock
  },
  {
    id: 'watermark',
    view: 'watermark',
    sidebar: true,
    group: 'Edit',
    name: 'Watermark',
    description: 'Stamp text or a logo on pages.',
    icon: Stamp
  },
  {
    id: 'properties',
    view: 'properties',
    sidebar: true,
    group: 'Edit',
    name: 'Properties',
    description: 'Edit title and author, or remove metadata.',
    icon: FileText
  }
];

/** The sidebar entries: one per screen. */
export const sidebarTools = tools.filter((t) => t.sidebar);
