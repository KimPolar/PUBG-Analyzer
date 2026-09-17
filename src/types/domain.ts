export interface AppSettings {
  username: string;
  platform: string;
  rpm: number;
  downloadConcurrency: number;
  cellSizeM: number;
  bucketSeconds: number;
  keepRawTelemetry: boolean;
}

export interface SettingsResponse {
  settings: AppSettings;
  apiKeyConfigured: boolean;
}

export interface StoredMatchSummary {
  matchId: string;
  createdAt: string | null;
  mapName: string | null;
  gameMode: string | null;
  status: string;
  error: string | null;
}

export interface DatasetCell {
  phaseCellId: string;
  cellId: string;
  phase: number;
  cellX: number;
  cellY: number;
  centerXM: number;
  centerYM: number;
  matchCount: number;
  occupancySamples: number;
  visits: number;
  survival120s: number | null;
  nextZoneRetention: number | null;
  meanHoldSeconds: number | null;
  meanEnemyTeams300m: number;
  damageBalance: number;
  historicalValueScore: number;
  scoreConfidence: number;
}

export interface MapDatasetAnalysis {
  mapName: string;
  matchCount: number;
  cells: DatasetCell[];
}

export interface DatasetAnalysis {
  matchCount: number;
  trainingRowCount: number;
  maps: MapDatasetAnalysis[];
}

export interface DashboardData {
  playerName: string | null;
  platform: string | null;
  storedMatches: number;
  analyzedMatches: number;
  failedMatches: number;
  maps: Array<{ mapName: string; matches: number }>;
  latestMatches: StoredMatchSummary[];
  dataset: DatasetAnalysis | null;
}

export type CollectionProgress =
  | { stage: "resolvingPlayer" }
  | { stage: "matchesDiscovered"; total: number; fresh: number }
  | { stage: "fetchingMatch"; current: number; total: number }
  | { stage: "downloadingTelemetry"; current: number; total: number }
  | { stage: "analyzing"; current: number; total: number }
  | { stage: "matchComplete"; matchId: string }
  | { stage: "matchFailed"; matchId: string; message: string }
  | { stage: "complete" };

export interface CollectionReport {
  discoveredMatches: number;
  newMatches: number;
  analyzedMatches: number;
  skippedMatches: number;
  failedMatches: number;
}

export interface TrainingResult {
  report: {
    inputRows?: number;
    trainedAt?: string;
    models?: Array<{
      model: string;
      status: string;
      rows: number;
      evaluation?: {
        strategy?: string;
        brierScore?: number | null;
        logLoss?: number | null;
        rocAuc?: number | null;
      };
      reason?: string;
    }>;
  };
  modelDirectory: string;
}

export type ViewName = "dashboard" | "matches" | "training" | "settings";
