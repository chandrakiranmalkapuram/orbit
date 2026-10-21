import type { ActionCategory } from '../types/orbit';

export const CATEGORY_LABELS: Record<ActionCategory | 'all', string> = {
  all: 'All',
  format: 'Formats',
  transform: 'Transforms',
  optimize: 'Optimize',
  archive: 'Archive',
  utility: 'Utilities',
};

export const CATEGORY_ICONS: Record<ActionCategory, string> = {
  format: '◈',
  transform: '⤢',
  optimize: '⇣',
  archive: '▣',
  utility: '⌕',
};
