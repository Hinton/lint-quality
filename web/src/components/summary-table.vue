<script setup lang="ts">
import { ref, computed } from "vue";
import type { DeltaRow } from "../types";

const CAP = 20;

const props = defineProps<{ rows: DeltaRow[] }>();
const expanded = ref(false);

const visibleRows = computed(() =>
  expanded.value ? props.rows : props.rows.slice(0, CAP),
);
const hasMore = computed(() => props.rows.length > CAP);
</script>

<template>
  <div v-if="rows.length > 0" class="overflow-x-auto">
    <table class="w-full text-sm">
      <thead>
        <tr class="border-b border-gray-200 text-left text-gray-500">
          <th class="py-2 pr-4 font-medium">Name</th>
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
          <td class="py-1.5 pr-4 text-gray-900 truncate max-w-xs">
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
            :class="{
              'text-green-600': row.delta < 0,
              'text-red-600': row.delta > 0,
              'text-gray-400': row.delta === 0,
            }"
          >
            {{ row.delta > 0 ? "+" : "" }}{{ row.delta }}
          </td>
          <td
            class="py-1.5 pl-3 text-right tabular-nums"
            :class="{
              'text-green-600': row.delta < 0,
              'text-red-600': row.delta > 0,
              'text-gray-400': row.delta === 0,
            }"
          >
            {{ row.delta > 0 ? "+" : "" }}{{ row.percent.toFixed(1) }}%
          </td>
        </tr>
      </tbody>
    </table>
    <button
      v-if="hasMore"
      class="mt-2 text-sm text-blue-600 hover:underline"
      @click="expanded = !expanded"
    >
      {{ expanded ? "Show less" : `Show all ${rows.length} rows` }}
    </button>
  </div>
</template>
