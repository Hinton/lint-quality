import { describe, it, expect } from "vitest";
import { ref } from "vue";
import type { Report, FileReport, Filter } from "../types";
import {
  ancestorDirs,
  aggregateFiles,
  availableValuesForDimension,
  useTrends,
} from "./use-trends";

// -- Helpers --

function makeViolation(overrides: Partial<FileReport["violations"][0]> = {}) {
  return {
    line: 1,
    pattern: "eslint-disable-next-line",
    category: "eslint",
    rules: ["no-unused-vars"],
    raw_text: "// eslint-disable-next-line no-unused-vars",
    ...overrides,
  };
}

function makeFile(overrides: Partial<FileReport> = {}): FileReport {
  return {
    path: "src/app.ts",
    violations: [makeViolation()],
    ...overrides,
  };
}

function makeSummary(files: FileReport[]) {
  return aggregateFiles(files, []);
}

function makeReport(
  files: FileReport[],
  timestamp = "2025-01-01T00:00:00Z",
): Report {
  return {
    metadata: {
      timestamp,
      tool_version: "0.1.0",
      scanned_paths: ["."],
      files_scanned: 10,
      scan_duration_ms: 100,
    },
    files,
    summary: {
      ...makeSummary(files),
      total_files_with_violations: files.length,
    },
  };
}

// -- Tests --

describe("ancestorDirs", () => {
  it("returns all ancestor directories", () => {
    expect(ancestorDirs("src/components/Button.vue")).toEqual([
      "src/components",
      "src",
    ]);
  });

  it("returns single parent for shallow path", () => {
    expect(ancestorDirs("src/app.ts")).toEqual(["src"]);
  });

  it("returns empty for file with no directory", () => {
    expect(ancestorDirs("app.ts")).toEqual([]);
  });

  it("handles deeply nested paths", () => {
    expect(ancestorDirs("a/b/c/d/file.ts")).toEqual([
      "a/b/c/d",
      "a/b/c",
      "a/b",
      "a",
    ]);
  });
});

describe("aggregateFiles", () => {
  it("counts total violations", () => {
    const files = [
      makeFile({ violations: [makeViolation(), makeViolation()] }),
      makeFile({ path: "src/b.ts", violations: [makeViolation()] }),
    ];
    const result = aggregateFiles(files, []);
    expect(result.total_violations).toBe(3);
  });

  it("aggregates by pattern", () => {
    const files = [
      makeFile({
        violations: [
          makeViolation({ pattern: "eslint-disable-next-line" }),
          makeViolation({ pattern: "ts-ignore" }),
        ],
      }),
    ];
    const result = aggregateFiles(files, []);
    expect(result.by_pattern).toEqual({
      "eslint-disable-next-line": 1,
      "ts-ignore": 1,
    });
  });

  it("aggregates by category", () => {
    const files = [
      makeFile({
        violations: [
          makeViolation({ category: "eslint" }),
          makeViolation({ category: "typescript" }),
          makeViolation({ category: "eslint" }),
        ],
      }),
    ];
    const result = aggregateFiles(files, []);
    expect(result.by_category).toEqual({ eslint: 2, typescript: 1 });
  });

  it("aggregates by rule across multiple rules per violation", () => {
    const files = [
      makeFile({
        violations: [
          makeViolation({ rules: ["no-unused-vars", "no-undef"] }),
          makeViolation({ rules: ["no-unused-vars"] }),
        ],
      }),
    ];
    const result = aggregateFiles(files, []);
    expect(result.by_rule).toEqual({
      "no-unused-vars": 2,
      "no-undef": 1,
    });
  });

  it("uses @unowned for files without an owner", () => {
    const files = [makeFile({ owner: undefined })];
    const result = aggregateFiles(files, []);
    expect(result.by_owner).toEqual({ "@unowned": 1 });
  });

  it("uses actual owner when present", () => {
    const files = [makeFile({ owner: "@team-foo" })];
    const result = aggregateFiles(files, []);
    expect(result.by_owner).toEqual({ "@team-foo": 1 });
  });

  it("counts directories hierarchically", () => {
    const files = [
      makeFile({ path: "src/components/Button.vue" }),
    ];
    const result = aggregateFiles(files, []);
    expect(result.by_directory).toEqual({
      "src/components": 1,
      src: 1,
    });
  });

  describe("with filters", () => {
    const files: FileReport[] = [
      makeFile({
        path: "src/a.ts",
        owner: "@team-a",
        violations: [
          makeViolation({ pattern: "eslint-disable-next-line", category: "eslint", rules: ["no-unused-vars"] }),
          makeViolation({ pattern: "ts-ignore", category: "typescript", rules: ["*"] }),
        ],
      }),
      makeFile({
        path: "lib/b.ts",
        owner: "@team-b",
        violations: [
          makeViolation({ pattern: "eslint-disable-next-line", category: "eslint", rules: ["no-undef"] }),
        ],
      }),
    ];

    it("filters by owner", () => {
      const filters: Filter[] = [
        { id: "1", dimension: "owner", values: new Set(["@team-a"]) },
      ];
      const result = aggregateFiles(files, filters);
      expect(result.total_violations).toBe(2);
      expect(result.by_owner).toEqual({ "@team-a": 2 });
    });

    it("filters by pattern", () => {
      const filters: Filter[] = [
        { id: "1", dimension: "pattern", values: new Set(["ts-ignore"]) },
      ];
      const result = aggregateFiles(files, filters);
      expect(result.total_violations).toBe(1);
      expect(result.by_pattern).toEqual({ "ts-ignore": 1 });
    });

    it("filters by category", () => {
      const filters: Filter[] = [
        { id: "1", dimension: "category", values: new Set(["typescript"]) },
      ];
      const result = aggregateFiles(files, filters);
      expect(result.total_violations).toBe(1);
    });

    it("filters by rule", () => {
      const filters: Filter[] = [
        { id: "1", dimension: "rule", values: new Set(["no-undef"]) },
      ];
      const result = aggregateFiles(files, filters);
      expect(result.total_violations).toBe(1);
      expect(result.by_owner).toEqual({ "@team-b": 1 });
    });

    it("filters by directory", () => {
      const filters: Filter[] = [
        { id: "1", dimension: "directory", values: new Set(["src"]) },
      ];
      const result = aggregateFiles(files, filters);
      expect(result.total_violations).toBe(2);
      expect(result.by_owner).toEqual({ "@team-a": 2 });
    });

    it("combines owner + pattern filters (AND logic)", () => {
      const filters: Filter[] = [
        { id: "1", dimension: "owner", values: new Set(["@team-a"]) },
        { id: "2", dimension: "pattern", values: new Set(["eslint-disable-next-line"]) },
      ];
      const result = aggregateFiles(files, filters);
      expect(result.total_violations).toBe(1);
      expect(result.by_rule).toEqual({ "no-unused-vars": 1 });
    });

    it("returns empty when filter matches nothing", () => {
      const filters: Filter[] = [
        { id: "1", dimension: "owner", values: new Set(["@nobody"]) },
      ];
      const result = aggregateFiles(files, filters);
      expect(result.total_violations).toBe(0);
    });

    it("ignores filters with empty values", () => {
      const filters: Filter[] = [
        { id: "1", dimension: "owner", values: new Set() },
      ];
      const result = aggregateFiles(files, filters);
      expect(result.total_violations).toBe(3);
    });
  });
});

describe("availableValuesForDimension", () => {
  const reports = [
    makeReport([
      makeFile({ owner: "@team-a", violations: [makeViolation({ rules: ["rule-a"] })] }),
    ]),
    makeReport([
      makeFile({ owner: "@team-b", violations: [makeViolation({ rules: ["rule-b"] })] }),
    ]),
  ];

  it("collects owner values across reports", () => {
    expect(availableValuesForDimension(reports, "owner")).toEqual([
      "@team-a",
      "@team-b",
    ]);
  });

  it("collects rule values across reports", () => {
    const values = availableValuesForDimension(reports, "rule");
    expect(values).toContain("rule-a");
    expect(values).toContain("rule-b");
  });

  it("returns sorted values", () => {
    const values = availableValuesForDimension(reports, "owner");
    expect(values).toEqual([...values].sort());
  });
});

describe("useTrends", () => {
  const fileA = makeFile({
    path: "src/a.ts",
    owner: "@team-a",
    violations: [
      makeViolation({ pattern: "eslint-disable-next-line", category: "eslint", rules: ["no-unused-vars"] }),
    ],
  });
  const fileB = makeFile({
    path: "lib/b.ts",
    owner: "@team-b",
    violations: [
      makeViolation({ pattern: "ts-ignore", category: "typescript", rules: ["*"] }),
      makeViolation({ pattern: "ts-ignore", category: "typescript", rules: ["*"] }),
    ],
  });

  it("returns 6 sections (total + 5 dimensions)", () => {
    const reports = ref([makeReport([fileA, fileB])]);
    const filters = ref<Filter[]>([]);
    const { sections } = useTrends(reports, filters);
    expect(sections.value).toHaveLength(6);
    expect(sections.value.map((s) => s.key)).toEqual([
      "total",
      "owner",
      "category",
      "rule",
      "pattern",
      "directory",
    ]);
  });

  it("total section has correct violation count", () => {
    const reports = ref([makeReport([fileA, fileB])]);
    const filters = ref<Filter[]>([]);
    const { sections } = useTrends(reports, filters);
    const total = sections.value[0];
    expect(total.series[0].points[0].value).toBe(3);
  });

  it("returns empty sections for no reports", () => {
    const reports = ref<Report[]>([]);
    const filters = ref<Filter[]>([]);
    const { sections } = useTrends(reports, filters);
    expect(sections.value).toEqual([]);
  });

  it("computes delta rows for 2+ reports", () => {
    const r1 = makeReport([fileA], "2025-01-01T00:00:00Z");
    const r2 = makeReport([fileA, fileB], "2025-02-01T00:00:00Z");
    const reports = ref([r1, r2]);
    const filters = ref<Filter[]>([]);
    const { sections } = useTrends(reports, filters);
    const total = sections.value[0];
    expect(total.deltaRows).toHaveLength(1);
    expect(total.deltaRows[0].first).toBe(1);
    expect(total.deltaRows[0].last).toBe(3);
    expect(total.deltaRows[0].delta).toBe(2);
  });

  it("applies filters to all sections", () => {
    const reports = ref([makeReport([fileA, fileB])]);
    const filters = ref<Filter[]>([
      { id: "1", dimension: "owner", values: new Set(["@team-a"]) },
    ]);
    const { sections } = useTrends(reports, filters);
    const total = sections.value[0];
    expect(total.series[0].points[0].value).toBe(1);
  });

  it("uses pre-computed summary when no filter values selected", () => {
    const reports = ref([makeReport([fileA, fileB])]);
    const filters = ref<Filter[]>([
      { id: "1", dimension: "owner", values: new Set() },
    ]);
    const { sections } = useTrends(reports, filters);
    const total = sections.value[0];
    expect(total.series[0].points[0].value).toBe(3);
  });

  describe("insights", () => {
    it("reports decrease as positive", () => {
      const r1 = makeReport(
        [fileA, fileB],
        "2025-01-01T00:00:00Z",
      );
      const r2 = makeReport([fileA], "2025-02-01T00:00:00Z");
      const reports = ref([r1, r2]);
      const filters = ref<Filter[]>([]);
      const { insights } = useTrends(reports, filters);
      expect(insights.value[0].type).toBe("positive");
      expect(insights.value[0].text).toContain("decreased");
    });

    it("reports increase as negative", () => {
      const r1 = makeReport([fileA], "2025-01-01T00:00:00Z");
      const r2 = makeReport(
        [fileA, fileB],
        "2025-02-01T00:00:00Z",
      );
      const reports = ref([r1, r2]);
      const filters = ref<Filter[]>([]);
      const { insights } = useTrends(reports, filters);
      expect(insights.value[0].type).toBe("negative");
      expect(insights.value[0].text).toContain("increased");
    });

    it("returns empty for single report", () => {
      const reports = ref([makeReport([fileA])]);
      const filters = ref<Filter[]>([]);
      const { insights } = useTrends(reports, filters);
      expect(insights.value).toEqual([]);
    });
  });
});
