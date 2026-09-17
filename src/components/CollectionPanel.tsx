import type { CollectionProgress, CollectionReport } from "../types/domain";

interface CollectionPanelProps {
  apiKeyConfigured: boolean;
  username: string;
  collecting: boolean;
  progress: CollectionProgress | null;
  report: CollectionReport | null;
  onStart: () => void;
  onCancel: () => void;
  onOpenSettings: () => void;
}

export function CollectionPanel({
  apiKeyConfigured,
  username,
  collecting,
  progress,
  report,
  onStart,
  onCancel,
  onOpenSettings,
}: CollectionPanelProps) {
  const readiness = Boolean(username.trim()) && apiKeyConfigured;
  const currentProgress = progressValue(progress);
  return (
    <section className="panel collection-panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">DATA PIPELINE</span>
          <h2>텔레메트리 수집</h2>
        </div>
        <span className={collecting ? "status-pill running" : "status-pill"}>
          <i aria-hidden="true" />
          {collecting ? "수집 중" : readiness ? "준비됨" : "설정 필요"}
        </span>
      </div>

      <div className="pipeline-status">
        <div className="progress-track" aria-label="수집 진행률">
          <span style={{ width: `${currentProgress}%` }} />
        </div>
        <div className="pipeline-copy">
          <strong>{progressLabel(progress, collecting)}</strong>
          <span>{progressDetail(progress, report)}</span>
        </div>
      </div>

      <div className="collection-actions">
        {collecting ? (
          <button className="button secondary" type="button" onClick={onCancel}>
            작업 중지
          </button>
        ) : (
          <button className="button primary" type="button" disabled={!readiness} onClick={onStart}>
            최신 매치 동기화
          </button>
        )}
        {!readiness ? (
          <button className="text-button" type="button" onClick={onOpenSettings}>
            계정과 API 키 설정 →
          </button>
        ) : (
          <span className="retention-note">PUBG API가 제공하는 최근 매치를 자동 확인합니다.</span>
        )}
      </div>
    </section>
  );
}

function progressValue(progress: CollectionProgress | null): number {
  if (!progress) return 0;
  switch (progress.stage) {
    case "resolvingPlayer":
      return 8;
    case "matchesDiscovered":
      return 15;
    case "fetchingMatch":
      return 15 + (progress.current / Math.max(1, progress.total)) * 20;
    case "downloadingTelemetry":
      return 35 + (progress.current / Math.max(1, progress.total)) * 30;
    case "analyzing":
      return 65 + (progress.current / Math.max(1, progress.total)) * 34;
    case "matchComplete":
    case "matchFailed":
      return 99;
    case "complete":
      return 100;
  }
}

function progressLabel(progress: CollectionProgress | null, collecting: boolean): string {
  if (!progress) return collecting ? "수집 작업 준비 중" : "새로운 매치를 확인할 수 있습니다";
  switch (progress.stage) {
    case "resolvingPlayer":
      return "플레이어 계정 확인 중";
    case "matchesDiscovered":
      return `${progress.total}개 매치 발견`;
    case "fetchingMatch":
      return "매치 메타데이터 수집 중";
    case "downloadingTelemetry":
      return "텔레메트리 다운로드 중";
    case "analyzing":
      return "Rust 분석 엔진 실행 중";
    case "matchComplete":
      return "매치 분석 완료";
    case "matchFailed":
      return "일부 매치 처리 실패";
    case "complete":
      return "동기화 완료";
  }
}

function progressDetail(
  progress: CollectionProgress | null,
  report: CollectionReport | null,
): string {
  if (!progress && report) {
    return `신규 분석 ${report.analyzedMatches} · 건너뜀 ${report.skippedMatches} · 실패 ${report.failedMatches}`;
  }
  if (!progress) return "이미 분석한 매치는 건너뛰고 실패한 매치는 재시도합니다.";
  if ("current" in progress) return `${progress.current.toLocaleString()} / ${progress.total.toLocaleString()}`;
  if (progress.stage === "matchesDiscovered") return `새로 처리할 매치 ${progress.fresh}개`;
  if (progress.stage === "matchFailed") return progress.message;
  return "로컬 SQLite에 안전하게 저장됩니다.";
}
