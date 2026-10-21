export type FileKind =
  | 'image'
  | 'archive'
  | 'pdf'
  | 'document'
  | 'spreadsheet'
  | 'presentation'
  | 'audio'
  | 'video'
  | 'unknown';

export type ActionCategory = 'format' | 'transform' | 'optimize' | 'archive' | 'utility';

export type ActionParameterType =
  | { type: 'quality'; min: number; max: number; default: number }
  | { type: 'scale'; presets: number[]; default: number }
  | { type: 'dimensions'; defaultWidth: number; defaultHeight: number }
  | { type: 'backgroundColor'; default: string };

export interface ActionDefinition {
  id: string;
  label: string;
  detail: string;
  symbol: string;
  category: ActionCategory;
  engine: string;
  inputFormats: string[];
  outputFormat?: string;
  available: boolean;
  limitations?: string;
  supportsBatch: boolean;
  supportsProgress: boolean;
  outputNamingRule: string;
  parameter?: ActionParameterType;
  isPrimary: boolean;
}

export interface FileMetadata {
  path: string;
  name: string;
  extension: string;
  kind: FileKind;
  mime: string;
  size: number;
  width?: number;
  height?: number;
  formatName?: string;
  hasAlpha?: boolean;
  actions: ActionDefinition[];
}

export interface ActionExecutionRequest {
  path: string;
  actionId: string;
  quality?: number;
  scalePercent?: number;
  targetWidth?: number;
  targetHeight?: number;
  bgColor?: string;
}

export interface OperationResult {
  outputPaths: string[];
  message: string;
  details?: string;
}
