import Merge from 'lucide-svelte/icons/merge';
import LayoutGrid from 'lucide-svelte/icons/layout-grid';
import Scissors from 'lucide-svelte/icons/scissors';
import FileOutput from 'lucide-svelte/icons/file-output';
import RotateCw from 'lucide-svelte/icons/rotate-cw';
import Scaling from 'lucide-svelte/icons/scaling';
import Minimize2 from 'lucide-svelte/icons/minimize-2';
import ImagePlus from 'lucide-svelte/icons/image-plus';
import type { ViewId } from './stores/nav.svelte';

export interface ToolDef {
  id: Exclude<ViewId, 'home' | 'about'>;
  name: string;
  description: string;
  icon: typeof Merge;
}

/** The PDF tools, in sidebar order. */
export const tools: ToolDef[] = [
  {
    id: 'merge',
    name: 'Merge',
    description: 'Combine several PDFs into one document.',
    icon: Merge
  },
  {
    id: 'pages',
    name: 'Pages',
    description: 'Reorder, rotate, delete or insert pages.',
    icon: LayoutGrid
  },
  {
    id: 'split',
    name: 'Split',
    description: 'Cut a PDF into several smaller files.',
    icon: Scissors
  },
  {
    id: 'extract',
    name: 'Extract',
    description: 'Pull selected pages out into a new PDF.',
    icon: FileOutput
  },
  {
    id: 'rotate',
    name: 'Rotate',
    description: 'Turn all or some pages, losslessly.',
    icon: RotateCw
  },
  {
    id: 'resize',
    name: 'Resize',
    description: 'Change page size to A4, Letter and more.',
    icon: Scaling
  },
  {
    id: 'compress',
    name: 'Compress',
    description: 'Make a PDF smaller for email and storage.',
    icon: Minimize2
  },
  {
    id: 'images',
    name: 'Images to PDF',
    description: 'Turn JPG and PNG images into a PDF.',
    icon: ImagePlus
  }
];
