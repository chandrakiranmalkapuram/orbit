export type FileKind = 'image' | 'archive' | 'pdf' | 'document' | 'spreadsheet' | 'presentation' | 'audio' | 'video' | 'unknown';
export type ActionId = 'to-png' | 'to-jpeg' | 'to-webp' | 'to-pdf' | 'compress-image' | 'extract-zip' | 'preview-zip' | 'create-zip';
export interface DetectedFile { path: string; name: string; extension: string; kind: FileKind; mime: string; size: number; actions: ActionId[] }
export interface ActionOption { id: ActionId; label: string; detail: string; symbol: string }
export interface OperationResult { outputPaths: string[]; message: string }
