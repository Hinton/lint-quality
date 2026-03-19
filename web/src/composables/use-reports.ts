import { ref } from "vue";
import type { Report } from "../types";

declare global {
  interface Window {
    __REPORTS__?: Report[];
  }
}

const reports = ref<Report[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

export function useReports() {
  function loadReports() {
    loading.value = true;
    error.value = null;
    try {
      const embedded = window.__REPORTS__;
      if (embedded && Array.isArray(embedded) && embedded.length > 0) {
        reports.value = embedded;
      }
    } catch (e) {
      error.value =
        e instanceof Error ? e.message : "Failed to load reports";
    } finally {
      loading.value = false;
    }
  }

  async function uploadReports(files: FileList) {
    const newReports: Report[] = [];
    for (const file of files) {
      try {
        const text = await file.text();
        const parsed = JSON.parse(text);
        if (Array.isArray(parsed)) {
          newReports.push(...parsed);
        } else {
          newReports.push(parsed);
        }
      } catch {
        console.warn(`Skipping invalid file: ${file.name}`);
      }
    }
    if (newReports.length > 0) {
      const all = [...reports.value, ...newReports];
      all.sort(
        (a, b) =>
          new Date(a.metadata.timestamp).getTime() -
          new Date(b.metadata.timestamp).getTime(),
      );
      reports.value = all;
    }
  }

  return { reports, loading, error, loadReports, uploadReports };
}
