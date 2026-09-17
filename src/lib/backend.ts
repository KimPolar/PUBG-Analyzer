import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { mockDashboard, mockSettings } from "./mock-data";
import type {
  CollectionProgress,
  CollectionReport,
  DashboardData,
  SettingsResponse,
  TrainingResult,
} from "../types/domain";

const inTauri = "__TAURI_INTERNALS__" in window;

export async function loadInitialData(): Promise<{
  settings: SettingsResponse;
  dashboard: DashboardData;
}> {
  if (!inTauri) return { settings: mockSettings, dashboard: mockDashboard };
  const [settings, dashboard] = await Promise.all([
    invoke<SettingsResponse>("get_settings"),
    invoke<DashboardData>("get_dashboard"),
  ]);
  return { settings, dashboard };
}

export function refreshDashboard(): Promise<DashboardData> {
  return inTauri ? invoke("get_dashboard") : Promise.resolve(mockDashboard);
}

export function persistSettings(
  settings: SettingsResponse["settings"],
  apiKey: string | null,
): Promise<SettingsResponse> {
  if (!inTauri) {
    return Promise.resolve({
      settings,
      apiKeyConfigured: apiKey ? true : mockSettings.apiKeyConfigured,
    });
  }
  return invoke("save_settings", { request: { settings, apiKey } });
}

export function collectMatches(): Promise<CollectionReport> {
  if (!inTauri) {
    return Promise.resolve({
      discoveredMatches: 36,
      newMatches: 0,
      analyzedMatches: 0,
      skippedMatches: 36,
      failedMatches: 0,
    });
  }
  return invoke("start_collection");
}

export function cancelCollection(): Promise<void> {
  return inTauri ? invoke("cancel_collection") : Promise.resolve();
}

export function trainModels(): Promise<TrainingResult> {
  if (!inTauri) {
    return Promise.resolve({
      report: {
        inputRows: 72_481,
        trainedAt: new Date().toISOString(),
        models: [
          { model: "next_zone_contains", status: "trained", rows: 72_481 },
          { model: "survives_60s", status: "trained", rows: 70_112 },
          { model: "survives_120s", status: "trained", rows: 65_873 },
        ],
      },
      modelDirectory: "AppData/PUBG Analyzer/models",
    });
  }
  return invoke("train_models");
}

export async function subscribeToCollection(
  handler: (event: CollectionProgress) => void,
): Promise<UnlistenFn> {
  if (!inTauri) return () => undefined;
  return listen<CollectionProgress>("collection-progress", (event) => handler(event.payload));
}
