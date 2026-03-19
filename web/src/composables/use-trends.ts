import { computed, type Ref } from "vue";
import type {
  Report,
  FileReport,
  Violation,
  Dimension,
  Filter,
  FilterDimension,
  DeltaRow,
  DimensionSection,
  Insight,
} from "../types";

type SummaryKey =
  | "by_pattern"
  | "by_category"
  | "by_rule"
  | "by_directory"
  | "by_owner";

const dimensionToKey: Record<Exclude<Dimension, "total">, SummaryKey> = {
  pattern: "by_pattern",
  category: "by_category",
  rule: "by_rule",
  directory: "by_directory",
  owner: "by_owner",
};

const dimensionLabels: Record<Dimension, string> = {
  total: "Total",
  owner: "By Owner",
  category: "By Category",
  rule: "By Rule",
  pattern: "By Pattern",
  directory: "By Directory",
};

/** Extract all ancestor directories from a file path (hierarchical, matching Rust backend) */
export function ancestorDirs(path: string): string[] {
  const dirs: string[] = [];
  const i = path.lastIndexOf("/");
  if (i < 0) return dirs;
  let dir = path.substring(0, i);
  while (dir.length > 0) {
    dirs.push(dir);
    const slash = dir.lastIndexOf("/");
    if (slash < 0) break;
    dir = dir.substring(0, slash);
  }
  return dirs;
}

/** Check whether a file+violations passes a single filter */
function matchesFilter(
  file: FileReport,
  violations: Violation[],
  filter: Filter,
): boolean {
  if (filter.values.size === 0) return true;

  switch (filter.dimension) {
    case "owner":
      return filter.values.has(file.owner ?? "@unowned");
    case "directory":
      return ancestorDirs(file.path).some((d) => filter.values.has(d));
    case "pattern":
      return violations.some((v) => filter.values.has(v.pattern));
    case "category":
      return violations.some((v) => filter.values.has(v.category));
    case "rule":
      return violations.some((v) => v.rules.some((r) => filter.values.has(r)));
  }
}

/** Filter violations within a file based on violation-level filters */
function filterViolations(violations: Violation[], filters: Filter[]): Violation[] {
  return violations.filter((v) => {
    for (const f of filters) {
      if (f.values.size === 0) continue;
      if (f.dimension === "pattern" && !f.values.has(v.pattern)) return false;
      if (f.dimension === "category" && !f.values.has(v.category)) return false;
      if (f.dimension === "rule" && !v.rules.some((r) => f.values.has(r)))
        return false;
    }
    return true;
  });
}

interface AggregatedSummary {
  total_violations: number;
  by_pattern: Record<string, number>;
  by_category: Record<string, number>;
  by_rule: Record<string, number>;
  by_directory: Record<string, number>;
  by_owner: Record<string, number>;
}

/** Re-aggregate from filtered file data */
export function aggregateFiles(
  files: FileReport[],
  filters: Filter[],
): AggregatedSummary {
  const summary: AggregatedSummary = {
    total_violations: 0,
    by_pattern: {},
    by_category: {},
    by_rule: {},
    by_directory: {},
    by_owner: {},
  };

  const fileFilters = filters.filter(
    (f) => f.dimension === "owner" || f.dimension === "directory",
  );
  const violationFilters = filters.filter(
    (f) =>
      f.dimension === "pattern" ||
      f.dimension === "category" ||
      f.dimension === "rule",
  );

  for (const file of files) {
    // Check file-level filters
    if (
      fileFilters.some(
        (f) => f.values.size > 0 && !matchesFilter(file, file.violations, f),
      )
    )
      continue;

    const violations = filterViolations(file.violations, violationFilters);
    if (violations.length === 0) continue;

    summary.total_violations += violations.length;

    for (const dir of ancestorDirs(file.path)) {
      summary.by_directory[dir] = (summary.by_directory[dir] ?? 0) + violations.length;
    }

    const owner = file.owner ?? "@unowned";
    summary.by_owner[owner] = (summary.by_owner[owner] ?? 0) + violations.length;

    for (const v of violations) {
      summary.by_pattern[v.pattern] =
        (summary.by_pattern[v.pattern] ?? 0) + 1;
      summary.by_category[v.category] =
        (summary.by_category[v.category] ?? 0) + 1;
      for (const r of v.rules) {
        summary.by_rule[r] = (summary.by_rule[r] ?? 0) + 1;
      }
    }
  }

  return summary;
}

/** Get the summary for a report, re-aggregating if filters are active */
function getSummary(report: Report, filters: Filter[]): AggregatedSummary {
  const active = filters.filter((f) => f.values.size > 0);
  if (active.length === 0) return report.summary;
  return aggregateFiles(report.files, active);
}

/** Collect all available values for a given filter dimension across reports */
export function availableValuesForDimension(
  reports: Report[],
  dimension: FilterDimension,
): string[] {
  const values = new Set<string>();
  for (const r of reports) {
    if (dimension === "owner" || dimension === "directory") {
      for (const k of Object.keys(r.summary[dimensionToKey[dimension]])) {
        values.add(k);
      }
    } else {
      for (const k of Object.keys(r.summary[dimensionToKey[dimension]])) {
        values.add(k);
      }
    }
  }
  return [...values].sort();
}

export function useTrends(
  reports: Ref<Report[]>,
  filters: Ref<Filter[]>,
) {
  /** Summaries for each report after applying filters */
  const filteredSummaries = computed(() =>
    reports.value.map((r) => getSummary(r, filters.value)),
  );

  /** Build sections for all dimensions */
  const sections = computed<DimensionSection[]>(() => {
    if (reports.value.length === 0) return [];
    const summaries = filteredSummaries.value;

    const result: DimensionSection[] = [];

    // Total section
    result.push({
      key: "total",
      label: dimensionLabels.total,
      series: [
        {
          label: "Total Violations",
          points: summaries.map((s, i) => ({
            timestamp: reports.value[i].metadata.timestamp,
            value: s.total_violations,
          })),
        },
      ],
      deltaRows: buildDeltaRows("total", summaries),
    });

    // Per-dimension sections
    for (const dim of ["owner", "category", "rule", "pattern", "directory"] as const) {
      const key = dimensionToKey[dim];
      const allKeys = new Set<string>();
      for (const s of summaries) {
        for (const k of Object.keys(s[key])) {
          allKeys.add(k);
        }
      }
      const sortedKeys = [...allKeys].sort();
      const topKeys = sortedKeys.slice(0, 10);

      result.push({
        key: dim,
        label: dimensionLabels[dim],
        series: topKeys.map((k) => ({
          label: k,
          points: summaries.map((s, i) => ({
            timestamp: reports.value[i].metadata.timestamp,
            value: s[key][k] ?? 0,
          })),
        })),
        deltaRows: buildDeltaRows(dim, summaries),
      });
    }

    return result;
  });

  function buildDeltaRows(
    dimension: Dimension,
    summaries: AggregatedSummary[],
  ): DeltaRow[] {
    if (summaries.length === 0) return [];
    const first = summaries.length >= 2 ? summaries[0] : summaries[0];
    const last = summaries[summaries.length - 1];
    const hasDelta = summaries.length >= 2;

    if (dimension === "total") {
      const f = hasDelta ? first.total_violations : last.total_violations;
      const l = last.total_violations;
      return [
        {
          label: "Total",
          first: f,
          last: l,
          delta: l - f,
          percent: f === 0 ? 0 : ((l - f) / f) * 100,
        },
      ];
    }

    const key = dimensionToKey[dimension];
    const allKeys = new Set<string>();
    if (hasDelta) {
      for (const k of Object.keys(first[key])) allKeys.add(k);
    }
    for (const k of Object.keys(last[key])) allKeys.add(k);

    const rows: DeltaRow[] = [];
    for (const k of allKeys) {
      const f = hasDelta ? (first[key][k] ?? 0) : (last[key][k] ?? 0);
      const l = last[key][k] ?? 0;
      rows.push({
        label: k,
        first: f,
        last: l,
        delta: l - f,
        percent: f === 0 ? (l > 0 ? 100 : 0) : ((l - f) / f) * 100,
      });
    }
    rows.sort((a, b) => a.delta - b.delta);
    return rows;
  }

  /** Auto-generated insights */
  const insights = computed<Insight[]>(() => {
    if (reports.value.length < 2) return [];
    const summaries = filteredSummaries.value;
    const first = summaries[0];
    const last = summaries[summaries.length - 1];
    const result: Insight[] = [];

    const totalFirst = first.total_violations;
    const totalLast = last.total_violations;
    const totalDelta = totalLast - totalFirst;
    const totalPct =
      totalFirst === 0 ? 0 : ((totalDelta / totalFirst) * 100).toFixed(1);

    if (totalDelta < 0) {
      result.push({
        text: `Total violations decreased by ${Math.abs(totalDelta)} (${totalFirst} \u2192 ${totalLast}, ${totalPct}%)`,
        type: "positive",
      });
    } else if (totalDelta > 0) {
      result.push({
        text: `Total violations increased by ${totalDelta} (${totalFirst} \u2192 ${totalLast}, +${totalPct}%)`,
        type: "negative",
      });
    } else {
      result.push({
        text: `Total violations unchanged at ${totalLast}`,
        type: "neutral",
      });
    }

    // Find biggest improver and regressor by owner
    const ownerFirst = first.by_owner;
    const ownerLast = last.by_owner;
    const allOwners = new Set([
      ...Object.keys(ownerFirst),
      ...Object.keys(ownerLast),
    ]);

    let bestOwner = { name: "", delta: 0 };
    let worstOwner = { name: "", delta: 0 };
    for (const owner of allOwners) {
      const d = (ownerLast[owner] ?? 0) - (ownerFirst[owner] ?? 0);
      if (d < bestOwner.delta) bestOwner = { name: owner, delta: d };
      if (d > worstOwner.delta) worstOwner = { name: owner, delta: d };
    }

    if (bestOwner.delta < 0) {
      result.push({
        text: `${bestOwner.name}: biggest improvement (${bestOwner.delta})`,
        type: "positive",
      });
    }
    if (worstOwner.delta > 0) {
      result.push({
        text: `${worstOwner.name}: biggest regression (+${worstOwner.delta})`,
        type: "negative",
      });
    }

    // Find the biggest rule regressor
    const ruleFirst = first.by_rule;
    const ruleLast = last.by_rule;
    const allRules = new Set([
      ...Object.keys(ruleFirst),
      ...Object.keys(ruleLast),
    ]);
    let worstRule = { name: "", delta: 0 };
    for (const rule of allRules) {
      const d = (ruleLast[rule] ?? 0) - (ruleFirst[rule] ?? 0);
      if (d > worstRule.delta) worstRule = { name: rule, delta: d };
    }
    if (worstRule.delta > 0) {
      result.push({
        text: `${worstRule.name}: largest rule increase (+${worstRule.delta})`,
        type: "negative",
      });
    }

    // Concentration
    if (Object.keys(ownerLast).length > 1 && totalLast > 0) {
      const topOwner = Object.entries(ownerLast).sort(
        ([, a], [, b]) => b - a,
      )[0];
      const pct = ((topOwner[1] / totalLast) * 100).toFixed(0);
      if (Number(pct) > 30) {
        result.push({
          text: `${topOwner[0]} accounts for ${pct}% of all violations`,
          type: "neutral",
        });
      }
    }

    return result;
  });

  return { sections, insights };
}
