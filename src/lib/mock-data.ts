import type { DashboardData, DatasetCell, SettingsResponse } from "../types/domain";

export const mockSettings: SettingsResponse = {
  settings: {
    username: "SamplePlayer",
    platform: "steam",
    rpm: 10,
    downloadConcurrency: 4,
    cellSizeM: 100,
    bucketSeconds: 10,
    keepRawTelemetry: true,
  },
  apiKeyConfigured: false,
};

function buildCells(): DatasetCell[] {
  const cells: DatasetCell[] = [];
  for (let y = 0; y < 10; y += 1) {
    for (let x = 0; x < 14; x += 1) {
      const distance = Math.hypot(x - 7.5, y - 4.5);
      if (distance > 6.6 || (x + y * 3) % 7 === 0) continue;
      const score = Math.max(28, Math.min(94, 91 - distance * 7 + ((x * 11 + y * 5) % 17)));
      cells.push({
        phaseCellId: `p3:${x}:${y}`,
        cellId: `${x}:${y}`,
        phase: 3,
        cellX: x,
        cellY: y,
        centerXM: x * 100 + 50,
        centerYM: y * 100 + 50,
        matchCount: 18 + ((x + y) % 12),
        occupancySamples: 34 + ((x * 5 + y * 7) % 90),
        visits: 9 + ((x + y * 2) % 24),
        survival120s: Math.min(0.96, score / 100 + 0.03),
        nextZoneRetention: Math.max(0.12, 0.78 - distance * 0.08),
        meanHoldSeconds: 42 + score * 0.9,
        meanEnemyTeams300m: Math.max(0.2, 2.9 - score / 42),
        damageBalance: score * 0.7 - 33,
        historicalValueScore: Number(score.toFixed(2)),
        scoreConfidence: Math.min(0.96, 0.48 + ((x + y) % 9) * 0.055),
      });
    }
  }
  return cells;
}

export const mockDashboard: DashboardData = {
  playerName: "SamplePlayer",
  platform: "steam",
  storedMatches: 36,
  analyzedMatches: 34,
  failedMatches: 2,
  maps: [
    { mapName: "Baltic_Main", matches: 21 },
    { mapName: "Desert_Main", matches: 13 },
  ],
  latestMatches: [
    {
      matchId: "5b7ec33f-6802-45b1-a991-3b97ff6b2a81",
      createdAt: "2026-09-17T01:57:00Z",
      mapName: "Baltic_Main",
      gameMode: "squad-fpp",
      status: "analyzed",
      error: null,
    },
    {
      matchId: "287d61d2-52dc-4272-b71c-09e07afc22b8",
      createdAt: "2026-09-16T22:14:00Z",
      mapName: "Desert_Main",
      gameMode: "squad-fpp",
      status: "analyzed",
      error: null,
    },
  ],
  dataset: {
    matchCount: 34,
    trainingRowCount: 72_481,
    maps: [{ mapName: "Baltic_Main", matchCount: 21, cells: buildCells() }],
  },
};
