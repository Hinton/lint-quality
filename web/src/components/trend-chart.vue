<script setup lang="ts">
import { computed } from "vue";
import { Line } from "vue-chartjs";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler,
} from "chart.js";
import type { TrendSeries } from "../types";

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler,
);

const props = defineProps<{ series: TrendSeries[] }>();

const COLORS = [
  "#3b82f6",
  "#ef4444",
  "#10b981",
  "#f59e0b",
  "#8b5cf6",
  "#ec4899",
  "#06b6d4",
  "#84cc16",
  "#f97316",
  "#6366f1",
];

const chartData = computed(() => {
  if (props.series.length === 0) return { labels: [], datasets: [] };

  const labels = props.series[0].points.map((p) => {
    const d = new Date(p.timestamp);
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  });

  const datasets = props.series.map((s, i) => ({
    label: s.label,
    data: s.points.map((p) => p.value),
    borderColor: COLORS[i % COLORS.length],
    backgroundColor: COLORS[i % COLORS.length] + "1a",
    tension: 0.3,
    fill: props.series.length === 1,
    pointRadius: 4,
    pointHoverRadius: 6,
  }));

  return { labels, datasets };
});

const chartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  interaction: {
    mode: "index" as const,
    intersect: false,
  },
  plugins: {
    legend: {
      display: true,
      position: "top" as const,
    },
    tooltip: {
      enabled: true,
    },
  },
  scales: {
    y: {
      beginAtZero: true,
      title: {
        display: true,
        text: "Violations",
      },
    },
  },
};
</script>

<template>
  <div class="h-80">
    <Line
      v-if="chartData.datasets.length > 0"
      :data="chartData"
      :options="chartOptions"
    />
    <div
      v-else
      class="flex items-center justify-center h-full text-gray-400"
    >
      No data to display
    </div>
  </div>
</template>
