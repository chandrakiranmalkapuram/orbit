import type { ActionOption, ActionId } from '../types/orbit';

export const ACTIONS: Record<ActionId, ActionOption> = {
  'to-png': { id: 'to-png', label: 'PNG', detail: 'Convert image', symbol: '◈' },
  'to-jpeg': { id: 'to-jpeg', label: 'JPEG', detail: 'Convert image', symbol: '◉' },
  'to-webp': { id: 'to-webp', label: 'WEBP', detail: 'Convert image', symbol: '◇' },
  'to-pdf': { id: 'to-pdf', label: 'PDF', detail: 'Create document', symbol: '▤' },
  'compress-image': { id: 'compress-image', label: 'Compress', detail: 'Reduce image size', symbol: '⇣' },
  'extract-zip': { id: 'extract-zip', label: 'Extract', detail: 'Unpack archive', symbol: '↗' },
  'preview-zip': { id: 'preview-zip', label: 'Contents', detail: 'Preview files', symbol: '☷' },
  'create-zip': { id: 'create-zip', label: 'Create ZIP', detail: 'Archive selected files', symbol: '▣' },
};
