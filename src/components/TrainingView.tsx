import type { DashboardData, TrainingResult } from "../types/domain";

interface TrainingViewProps {
  dashboard: DashboardData;
  training: boolean;
  result: TrainingResult | null;
  onTrain: () => void;
}

const modelDescriptions = [
  { name: "next_zone_contains", label: "다음 원 잔류", target: "후보 위치가 다음 안전구역에 포함될 확률" },
  { name: "survives_60s", label: "60초 생존", target: "현재 상태에서 팀 관측이 60초 뒤에도 유지될 확률" },
  { name: "survives_120s", label: "120초 생존", target: "장기 자리 유지력을 나타내는 120초 생존 확률" },
];

export function TrainingView({ dashboard, training, result, onTrain }: TrainingViewProps) {
  const rows = dashboard.dataset?.trainingRowCount ?? 0;
  const reports = new Map(result?.report.models?.map((model) => [model.model, model]));
  return (
    <div className="view-stack narrow-view">
      <header className="page-header training-header">
        <div>
          <span className="eyebrow">OPTIONAL PYTHON SIDECAR</span>
          <h1>확률 모델 학습</h1>
          <p>Rust가 만든 현재시점 feature를 매치 단위로 분리 검증한 뒤 모델 아티팩트로 저장합니다.</p>
        </div>
        <button className="button primary" type="button" disabled={training || rows === 0} onClick={onTrain}>
          {training ? "학습 중…" : "모델 다시 학습"}
        </button>
      </header>

      <section className="training-overview">
        <div><span>사용 가능한 행</span><strong>{rows.toLocaleString("ko-KR")}</strong></div>
        <div><span>분석 매치</span><strong>{dashboard.analyzedMatches.toLocaleString("ko-KR")}</strong></div>
        <div><span>검증 방식</span><strong>Match Group</strong></div>
        <div><span>런타임</span><strong>Bundled Python</strong></div>
      </section>

      <section className="panel model-list">
        <div className="panel-heading">
          <div><span className="eyebrow">MODEL REGISTRY</span><h2>학습 대상</h2></div>
          <span className="status-pill"><i /> 앱과 격리됨</span>
        </div>
        {modelDescriptions.map((model) => {
          const report = reports.get(model.name);
          return (
            <article className="model-row" key={model.name}>
              <div className="model-icon" aria-hidden="true">◇</div>
              <div className="model-copy"><strong>{model.label}</strong><span>{model.target}</span><code>{model.name}</code></div>
              <div className="model-metrics">
                {report ? (
                  <>
                    <span className={`model-status ${report.status}`}>{report.status === "trained" ? "TRAINED" : "SKIPPED"}</span>
                    <strong>{report.rows.toLocaleString("ko-KR")} rows</strong>
                    <small>{metricText(report.evaluation?.brierScore, "Brier")}</small>
                  </>
                ) : (
                  <><span className="model-status pending">PENDING</span><strong>—</strong><small>아직 학습하지 않음</small></>
                )}
              </div>
            </article>
          );
        })}
      </section>

      <section className="panel leakage-note">
        <div className="note-mark">!</div>
        <div>
          <strong>미래 정보 누수 차단</strong>
          <p>실제 다음 원 좌표와 미래 이동거리는 정답 생성에만 사용하고 모델 입력에서는 제외합니다. 검증 데이터도 행 단위가 아니라 매치 단위로 분리합니다.</p>
        </div>
      </section>

      {result ? (
        <section className="panel training-result">
          <span className="eyebrow">LAST TRAINING RUN</span>
          <strong>{result.report.trainedAt ? new Date(result.report.trainedAt).toLocaleString("ko-KR") : "완료"}</strong>
          <span>{result.modelDirectory}</span>
        </section>
      ) : null}
    </div>
  );
}

function metricText(value: number | null | undefined, label: string): string {
  return value === null || value === undefined ? "검증 지표 없음" : `${label} ${value.toFixed(4)}`;
}
