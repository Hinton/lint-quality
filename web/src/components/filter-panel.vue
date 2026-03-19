<script setup lang="ts">
import { ref } from "vue";
import type { Filter, FilterDimension } from "../types";

defineProps<{
  filters: Filter[];
  availableValues: (dimension: FilterDimension) => string[];
}>();

const emit = defineEmits<{
  (e: "add", dimension: FilterDimension): void;
  (e: "remove", id: string): void;
  (e: "toggle", id: string, value: string): void;
}>();

const dimensionOptions: { key: FilterDimension; label: string }[] = [
  { key: "owner", label: "Owner" },
  { key: "category", label: "Category" },
  { key: "rule", label: "Rule" },
  { key: "pattern", label: "Pattern" },
  { key: "directory", label: "Directory" },
];

const showMenu = ref(false);

function addFilter(dim: FilterDimension) {
  showMenu.value = false;
  emit("add", dim);
}
</script>

<template>
  <div class="flex flex-wrap items-start gap-3">
    <!-- Active filters -->
    <div
      v-for="filter in filters"
      :key="filter.id"
      class="bg-white border border-gray-200 rounded-lg p-3 min-w-[200px] max-w-xs"
    >
      <div class="flex items-center justify-between mb-2">
        <span class="text-xs font-semibold text-gray-500 uppercase tracking-wide">
          {{ filter.dimension }}
        </span>
        <button
          class="text-gray-400 hover:text-red-500 text-sm leading-none"
          title="Remove filter"
          @click="emit('remove', filter.id)"
        >
          &times;
        </button>
      </div>
      <div class="max-h-36 overflow-y-auto space-y-1">
        <label
          v-for="val in availableValues(filter.dimension)"
          :key="val"
          class="flex items-center gap-2 text-sm text-gray-600 cursor-pointer hover:text-gray-900"
        >
          <input
            type="checkbox"
            :checked="filter.values.has(val)"
            class="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
            @change="emit('toggle', filter.id, val)"
          />
          <span class="truncate">{{ val }}</span>
        </label>
      </div>
    </div>

    <!-- Add filter button -->
    <div class="relative">
      <button
        class="flex items-center gap-1 px-3 py-2 text-sm font-medium text-blue-600 border border-dashed border-blue-300 rounded-lg hover:bg-blue-50 transition-colors"
        @click="showMenu = !showMenu"
      >
        <span class="text-lg leading-none">+</span> Add Filter
      </button>
      <div
        v-if="showMenu"
        class="absolute top-full left-0 mt-1 bg-white border border-gray-200 rounded-lg shadow-lg py-1 z-10 min-w-[150px]"
      >
        <button
          v-for="opt in dimensionOptions"
          :key="opt.key"
          class="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-100"
          @click="addFilter(opt.key)"
        >
          {{ opt.label }}
        </button>
      </div>
    </div>
  </div>
</template>
