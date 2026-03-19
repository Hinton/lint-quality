<script setup lang="ts">
import type { Insight } from "../types";

defineProps<{ insights: Insight[] }>();

const icons: Record<Insight["type"], string> = {
  positive: "↓",
  negative: "↑",
  neutral: "→",
};

const colors: Record<Insight["type"], string> = {
  positive: "text-green-700 bg-green-50 border-green-200",
  negative: "text-red-700 bg-red-50 border-red-200",
  neutral: "text-gray-700 bg-gray-50 border-gray-200",
};
</script>

<template>
  <div v-if="insights.length > 0" class="space-y-2">
    <h3 class="text-sm font-semibold text-gray-700">Insights</h3>
    <div class="space-y-1.5">
      <div
        v-for="(insight, i) in insights"
        :key="i"
        class="flex items-start gap-2 px-3 py-2 rounded-lg border text-sm"
        :class="colors[insight.type]"
      >
        <span class="flex-shrink-0 font-bold">{{ icons[insight.type] }}</span>
        <span>{{ insight.text }}</span>
      </div>
    </div>
  </div>
</template>
