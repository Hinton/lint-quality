<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import type { Filter, FilterDimension } from "./types";
import { useReports } from "./composables/use-reports";
import { useTrends, availableValuesForDimension } from "./composables/use-trends";
import TrendChart from "./components/trend-chart.vue";
import FilterPanel from "./components/filter-panel.vue";
import SummaryTable from "./components/summary-table.vue";
import InsightsPanel from "./components/insights-panel.vue";
import DirectoryTree from "./components/directory-tree.vue";

const { reports, loading, error, loadReports, uploadReports } = useReports();

const filters = ref<Filter[]>([]);
let nextFilterId = 0;

function addFilter(dimension: FilterDimension) {
  filters.value = [
    ...filters.value,
    { id: String(nextFilterId++), dimension, values: new Set() },
  ];
}

function removeFilter(id: string) {
  filters.value = filters.value.filter((f) => f.id !== id);
}

function toggleFilterValue(id: string, value: string) {
  filters.value = filters.value.map((f) => {
    if (f.id !== id) return f;
    const next = new Set(f.values);
    if (next.has(value)) {
      next.delete(value);
    } else {
      next.add(value);
    }
    return { ...f, values: next };
  });
}

function getAvailableValues(dimension: FilterDimension): string[] {
  return availableValuesForDimension(reports.value, dimension);
}

const { sections, insights } = useTrends(reports, filters);

const dragging = ref(false);
let dragCounter = 0;

function handleDragEnter() {
  dragCounter++;
  dragging.value = true;
}

function handleDragLeave() {
  dragCounter--;
  if (dragCounter === 0) dragging.value = false;
}

function handleDrop(e: DragEvent) {
  e.preventDefault();
  dragCounter = 0;
  dragging.value = false;
  if (e.dataTransfer?.files) {
    uploadReports(e.dataTransfer.files);
  }
}

function handleFileInput(e: Event) {
  const input = e.target as HTMLInputElement;
  if (input.files) {
    uploadReports(input.files);
  }
}

const dateRange = computed(() => {
  if (reports.value.length === 0) return "";
  const first = new Date(reports.value[0].metadata.timestamp);
  const last = new Date(
    reports.value[reports.value.length - 1].metadata.timestamp,
  );
  const fmt = (d: Date) =>
    d.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  return `${fmt(first)} – ${fmt(last)}`;
});

const hasActiveFilters = computed(() =>
  filters.value.some((f) => f.values.size > 0),
);

onMounted(loadReports);
</script>

<template>
  <div
    class="min-h-screen bg-gray-50 text-gray-900 relative"
    @dragover.prevent
    @dragenter="handleDragEnter"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
  >
    <!-- Drop overlay -->
    <Transition name="fade">
      <div
        v-if="dragging"
        class="fixed inset-0 z-50 bg-blue-500/10 backdrop-blur-sm flex items-center justify-center pointer-events-none"
      >
        <div
          class="border-2 border-dashed border-blue-500 rounded-2xl px-12 py-10 bg-white/80 shadow-lg text-center"
        >
          <p class="text-2xl font-semibold text-blue-600">Drop JSON reports here</p>
          <p class="text-sm text-blue-400 mt-1">Files will be added to the dashboard</p>
        </div>
      </div>
    </Transition>

    <!-- Header -->
    <header class="bg-white border-b border-gray-200 px-6 py-4">
      <div class="max-w-7xl mx-auto flex items-center justify-between">
        <div>
          <h1 class="text-xl font-bold text-gray-900">lint-quality trends</h1>
          <p v-if="reports.length > 0" class="text-sm text-gray-500">
            {{ reports.length }} report{{ reports.length !== 1 ? "s" : "" }}
            <span v-if="dateRange"> &middot; {{ dateRange }}</span>
          </p>
        </div>
        <label
          class="cursor-pointer px-4 py-2 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 transition-colors"
        >
          Upload Reports
          <input
            type="file"
            accept=".json"
            multiple
            class="hidden"
            @change="handleFileInput"
          />
        </label>
      </div>
    </header>

    <main class="max-w-7xl mx-auto px-6 py-6 space-y-6">
      <!-- Loading / Error -->
      <div
        v-if="loading"
        class="flex items-center justify-center py-20 text-gray-400"
      >
        Loading reports...
      </div>
      <div
        v-else-if="error"
        class="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700"
      >
        {{ error }}
      </div>

      <!-- Dashboard -->
      <template v-else-if="reports.length > 0">
        <!-- Filters -->
        <div class="bg-white rounded-lg border border-gray-200 p-4">
          <div class="flex items-center justify-between mb-3">
            <h2 class="text-sm font-semibold text-gray-700">Filters</h2>
            <span
              v-if="hasActiveFilters"
              class="text-xs text-blue-600 bg-blue-50 px-2 py-0.5 rounded-full"
            >
              Filtered
            </span>
          </div>
          <FilterPanel
            :filters="filters"
            :available-values="getAvailableValues"
            @add="addFilter"
            @remove="removeFilter"
            @toggle="toggleFilterValue"
          />
        </div>

        <!-- Insights -->
        <div class="bg-white rounded-lg border border-gray-200 p-4">
          <InsightsPanel :insights="insights" />
        </div>

        <!-- Dimension sections -->
        <div
          v-for="section in sections"
          :key="section.key"
          class="space-y-4"
        >
          <h2 class="text-lg font-semibold text-gray-800">{{ section.label }}</h2>

          <div class="bg-white rounded-lg border border-gray-200 p-6">
            <TrendChart :series="section.series" />
          </div>

          <div
            v-if="section.deltaRows.length > 0"
            class="bg-white rounded-lg border border-gray-200 p-4"
          >
            <DirectoryTree
              v-if="section.key === 'directory'"
              :rows="section.deltaRows"
            />
            <SummaryTable v-else :rows="section.deltaRows" />
          </div>
        </div>
      </template>

      <!-- Empty state -->
      <div
        v-else
        class="flex flex-col items-center justify-center py-20 text-gray-400 space-y-3"
      >
        <p class="text-lg">No reports loaded</p>
        <p class="text-sm">
          Drag and drop JSON report files here, or use the Upload button
        </p>
      </div>
    </main>
  </div>
</template>
