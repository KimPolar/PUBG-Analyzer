import { useCallback, useEffect, useState } from "react";
import { DashboardView } from "./components/DashboardView";
import { MatchTable } from "./components/MatchTable";
import { SettingsView } from "./components/SettingsView";
import { Sidebar } from "./components/Sidebar";
import { TrainingView } from "./components/TrainingView";
import {
  cancelCollection,
  collectMatches,
  loadInitialData,
  persistSettings,
  refreshDashboard,
  subscribeToCollection,
  trainModels,
} from "./lib/backend";
import type {
  AppSettings,
  CollectionProgress,
  CollectionReport,
  DashboardData,
  SettingsResponse,
  TrainingResult,
  ViewName,
} from "./types/domain";

export default function App() {
  const [view, setView] = useState<ViewName>("dashboard");
  const [dashboard, setDashboard] = useState<DashboardData | null>(null);
  const [settings, setSettings] = useState<SettingsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [collecting, setCollecting] = useState(false);
  const [progress, setProgress] = useState<CollectionProgress | null>(null);
  const [report, setReport] = useState<CollectionReport | null>(null);
  const [saving, setSaving] = useState(false);
  const [training, setTraining] = useState(false);
  const [trainingResult, setTrainingResult] = useState<TrainingResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    void subscribeToCollection(setProgress).then((stop) => {
      if (disposed) stop();
      else unsubscribe = stop;
    });
    void loadInitialData()
      .then((initial) => {
        if (disposed) return;
        setSettings(initial.settings);
        setDashboard(initial.dashboard);
      })
      .catch((reason: unknown) => {
        if (!disposed) setError(errorMessage(reason));
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);

  const startCollection = useCallback(async () => {
    setCollecting(true);
    setError(null);
    setReport(null);
    setProgress({ stage: "resolvingPlayer" });
    try {
      const nextReport = await collectMatches();
      const nextDashboard = await refreshDashboard();
      setReport(nextReport);
      setDashboard(nextDashboard);
      setProgress(null);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setCollecting(false);
    }
  }, []);

  const stopCollection = useCallback(async () => {
    try {
      await cancelCollection();
    } catch (reason) {
      setError(errorMessage(reason));
    }
  }, []);

  const saveSettings = useCallback(async (next: AppSettings, apiKey: string | null) => {
    setSaving(true);
    setError(null);
    try {
      setSettings(await persistSettings(next, apiKey));
      setView("dashboard");
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setSaving(false);
    }
  }, []);

  const runTraining = useCallback(async () => {
    setTraining(true);
    setError(null);
    try {
      setTrainingResult(await trainModels());
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setTraining(false);
    }
  }, []);

  if (loading || !dashboard || !settings) return <LoadingScreen />;

  return (
    <div className="app-shell">
      <Sidebar
        activeView={view}
        playerName={dashboard.playerName ?? settings.settings.username ?? null}
        platform={dashboard.platform ?? settings.settings.platform}
        onNavigate={setView}
      />
      <main className="main-content">
        {error ? (
          <div className="error-banner" role="alert">
            <div><strong>작업을 완료하지 못했습니다</strong><span>{error}</span></div>
            <button type="button" onClick={() => setError(null)} aria-label="오류 닫기">×</button>
          </div>
        ) : null}
        {view === "dashboard" ? (
          <DashboardView
            dashboard={dashboard}
            apiKeyConfigured={settings.apiKeyConfigured}
            username={settings.settings.username}
            collecting={collecting}
            progress={progress}
            report={report}
            onStart={startCollection}
            onCancel={stopCollection}
            onOpenSettings={() => setView("settings")}
          />
        ) : null}
        {view === "matches" ? (
          <div className="view-stack">
            <header className="page-header"><div><span className="eyebrow">MATCH ARCHIVE</span><h1>매치 기록</h1><p>로컬 데이터셋의 처리 상태와 재시도 대상을 확인합니다.</p></div></header>
            <MatchTable matches={dashboard.latestMatches} expanded />
          </div>
        ) : null}
        {view === "training" ? (
          <TrainingView dashboard={dashboard} training={training} result={trainingResult} onTrain={runTraining} />
        ) : null}
        {view === "settings" ? <SettingsView value={settings} saving={saving} onSave={saveSettings} /> : null}
      </main>
    </div>
  );
}

function LoadingScreen() {
  return (
    <div className="loading-screen">
      <div className="brand-mark">PA</div>
      <strong>PUBG ANALYZER</strong>
      <span>Rust 분석 엔진 초기화 중…</span>
    </div>
  );
}

function errorMessage(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}
