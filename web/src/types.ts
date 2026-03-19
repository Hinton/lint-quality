export interface Violation {
  line: number;
  pattern: string;
  category: string;
  rules: string[];
  raw_text: string;
}

export interface FileReport {
  path: string;
  owner?: string;
  violations: Violation[];
}

export interface ReportMetadata {
  timestamp: string;
  tool_version: string;
  scanned_paths: string[];
  config_path?: string;
  files_scanned: number;
  scan_duration_ms: number;
}

export interface ReportSummary {
  total_violations: number;
  total_files_with_violations: number;
  by_pattern: Record<string, number>;
  by_category: Record<string, number>;
  by_rule: Record<string, number>;
  by_directory: Record<string, number>;
  by_owner: Record<string, number>;
}

export interface Report {
  metadata: ReportMetadata;
  files: FileReport[];
  summary: ReportSummary;
}

export type Dimension =
  | "total"
  | "owner"
  | "category"
  | "rule"
  | "directory"
  | "pattern";

export interface TrendPoint {
  timestamp: string;
  value: number;
}

export interface TrendSeries {
  label: string;
  points: TrendPoint[];
}

export interface DeltaRow {
  label: string;
  first: number;
  last: number;
  delta: number;
  percent: number;
}

export interface Insight {
  text: string;
  type: "positive" | "negative" | "neutral";
}

export type FilterDimension = Exclude<Dimension, "total">;

export interface Filter {
  id: string;
  dimension: FilterDimension;
  values: Set<string>;
}

export interface DimensionSection {
  key: Dimension;
  label: string;
  series: TrendSeries[];
  deltaRows: DeltaRow[];
}
