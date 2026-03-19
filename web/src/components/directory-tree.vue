<script setup lang="ts">
import { ref, computed } from "vue";
import type { DeltaRow } from "../types";

const props = defineProps<{ rows: DeltaRow[] }>();

const sorted = computed(() =>
  [...props.rows].sort((a, b) => a.label.localeCompare(b.label)),
);

const minDepth = computed(() =>
  sorted.value.reduce(
    (m, r) => Math.min(m, r.label.split("/").length),
    Infinity,
  ),
);

function depth(label: string): number {
  return label.split("/").length - minDepth.value;
}

function hasChildren(label: string): boolean {
  return sorted.value.some(
    (r) => r.label !== label && r.label.startsWith(label + "/"),
  );
}

/** Track which parent nodes are expanded (start collapsed) */
const expanded = ref<string[]>([]);

function isExpanded(label: string): boolean {
  return expanded.value.includes(label);
}

function isVisible(label: string): boolean {
  // A row at depth 0 is always visible
  if (depth(label) === 0) return true;
  // Otherwise, every ancestor must be expanded
  const parts = label.split("/");
  for (let i = minDepth.value; i < parts.length - 1; i++) {
    const ancestor = parts.slice(0, i + 1).join("/");
    if (!expanded.value.includes(ancestor)) return false;
  }
  return true;
}

const visibleRows = computed(() =>
  sorted.value.filter((r) => isVisible(r.label)),
);

function toggle(label: string) {
  if (expanded.value.includes(label)) {
    expanded.value = expanded.value.filter((l) => l !== label);
  } else {
    expanded.value = [...expanded.value, label];
  }
}

function expandAll() {
  expanded.value = sorted.value
    .filter((r) => hasChildren(r.label))
    .map((r) => r.label);
}

function collapseAll() {
  expanded.value = [];
}

function deltaClass(row: DeltaRow): string {
  if (row.delta < 0) return "text-green-600";
  if (row.delta > 0) return "text-red-600";
  return "text-gray-400";
}
</script>

<template>
  <div v-if="sorted.length > 0" class="overflow-x-auto">
    <div class="flex items-center justify-end gap-2 mb-2">
      <button class="text-xs text-blue-600 hover:underline" @click="expandAll">
        Expand all
      </button>
      <span class="text-gray-300">|</span>
      <button class="text-xs text-blue-600 hover:underline" @click="collapseAll">
        Collapse all
      </button>
    </div>
    <table class="w-full text-sm">
      <thead>
        <tr class="border-b border-gray-200 text-left text-gray-500">
          <th class="py-2 pr-4 font-medium">Directory</th>
          <th class="py-2 px-3 font-medium text-right">First</th>
          <th class="py-2 px-3 font-medium text-right">Latest</th>
          <th class="py-2 px-3 font-medium text-right">Delta</th>
          <th class="py-2 pl-3 font-medium text-right">Change</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="row in visibleRows"
          :key="row.label"
          class="border-b border-gray-100 hover:bg-gray-50"
        >
          <td
            class="py-1.5 pr-4 text-gray-900 truncate max-w-xs"
            :style="{ paddingLeft: `${depth(row.label) * 1.25}rem` }"
          >
            <button
              v-if="hasChildren(row.label)"
              class="inline-block w-4 text-gray-400 hover:text-gray-700"
              @click="toggle(row.label)"
            >
              {{ isExpanded(row.label) ? "\u25BE" : "\u25B8" }}
            </button>
            <span v-else class="inline-block w-4">&nbsp;</span>
            {{ row.label }}
          </td>
          <td class="py-1.5 px-3 text-right text-gray-600 tabular-nums">
            {{ row.first }}
          </td>
          <td class="py-1.5 px-3 text-right text-gray-600 tabular-nums">
            {{ row.last }}
          </td>
          <td
            class="py-1.5 px-3 text-right font-medium tabular-nums"
            :class="deltaClass(row)"
          >
            {{ row.delta > 0 ? "+" : "" }}{{ row.delta }}
          </td>
          <td
            class="py-1.5 pl-3 text-right tabular-nums"
            :class="deltaClass(row)"
          >
            {{ row.delta > 0 ? "+" : "" }}{{ row.percent.toFixed(1) }}%
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
