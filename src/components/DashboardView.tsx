import type { CollectionProgress, CollectionReport, DashboardData } from "../types/domain";
import { CollectionPanel } from "./CollectionPanel";
import { MatchTable } from "./MatchTable";
import { PositionHeatmap } from "./PositionHeatmap";
import { SummaryCards } from "./SummaryCards";

interface DashboardViewProps {
  dashboard: DashboardData;
  apiKeyConfigured: boolean;
  username: string;
  collecting: boolean;
  progress: CollectionProgress | null;
  report: CollectionReport | null;
  onStart: () => void;
  onCancel: () => void;
  onOpenSettings: () => void;
}

export function DashboardView(props: DashboardViewProps) {
  return (
    <div className="view-stack">
      <header className="page-header dashboard-header">
        <div>
          <span className="eyebrow">POSITION INTELLIGENCE</span>
          <h1>어디를 먹어야 유리한가</h1>
          <p>원 잔류 확률뿐 아니라 생존, 자리 유지, 경쟁도와 회전 결과를 함께 봅니다.</p>
        </div>
        <div className="engine-chip"><i /> RUST ENGINE ONLINE</div>
      </header>
      <SummaryCards dashboard={props.dashboard} />
      <div className="dashboard-top-grid">
        <CollectionPanel
          apiKeyConfigured={props.apiKeyConfigured}
          username={props.username}
          collecting={props.collecting}
          progress={props.progress}
          report={props.report}
          onStart={props.onStart}
          onCancel={props.onCancel}
          onOpenSettings={props.onOpenSettings}
        />
        <section className="panel methodology-panel">
          <span className="eyebrow">VALUE COMPOSITION</span>
          <h2>점수 구성</h2>
          <div className="method-list">
            <div><span>생존·자리 유지</span><i style={{ width: "82%" }} /><strong>핵심</strong></div>
            <div><span>다음 원 잔류</span><i style={{ width: "68%" }} /><strong>확률</strong></div>
            <div><span>주변 경쟁도</span><i style={{ width: "54%" }} /><strong>위험</strong></div>
            <div><span>교전 손익</span><i style={{ width: "47%" }} /><strong>결과</strong></div>
          </div>
          <small>지형 이름 대신 실제 이동·체류·피해 결과를 proxy로 사용합니다.</small>
        </section>
      </div>
      <PositionHeatmap dataset={props.dashboard.dataset} />
      <MatchTable matches={props.dashboard.latestMatches} />
    </div>
  );
}
