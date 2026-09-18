import { formatPubgMapName } from "../lib/pubg-maps";
import type { DashboardData } from "../types/domain";

interface SummaryCardsProps {
  dashboard: DashboardData;
}

export function SummaryCards({ dashboard }: SummaryCardsProps) {
  const trainingRows = dashboard.dataset?.trainingRowCount ?? 0;
  const mapCount = dashboard.maps.length;
  const cards = [
    {
      label: "분석 완료",
      value: dashboard.analyzedMatches.toLocaleString("ko-KR"),
      detail: `전체 ${dashboard.storedMatches.toLocaleString("ko-KR")} 매치`,
      tone: "mint",
    },
    {
      label: "학습 행",
      value: compactNumber(trainingRows),
      detail: "팀 단위 시점 스냅샷",
      tone: "amber",
    },
    {
      label: "맵 커버리지",
      value: `${mapCount}`,
      detail: dashboard.maps.slice(0, 2).map((map) => formatPubgMapName(map.mapName)).join(" · ") || "대기 중",
      tone: "blue",
    },
    {
      label: "처리 실패",
      value: dashboard.failedMatches.toLocaleString("ko-KR"),
      detail: dashboard.failedMatches > 0 ? "다음 수집에서 자동 재시도" : "파이프라인 정상",
      tone: dashboard.failedMatches > 0 ? "red" : "neutral",
    },
  ];

  return (
    <section className="summary-grid" aria-label="분석 현황">
      {cards.map((card) => (
        <article className={`summary-card ${card.tone}`} key={card.label}>
          <span>{card.label}</span>
          <strong>{card.value}</strong>
          <small>{card.detail}</small>
        </article>
      ))}
    </section>
  );
}

function compactNumber(value: number): string {
  return new Intl.NumberFormat("ko-KR", {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}
