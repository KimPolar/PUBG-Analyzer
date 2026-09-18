import { formatPubgMapName } from "../lib/pubg-maps";
import type { StoredMatchSummary } from "../types/domain";

interface MatchTableProps {
  matches: StoredMatchSummary[];
  expanded?: boolean;
}

export function MatchTable({ matches, expanded = false }: MatchTableProps) {
  const visible = expanded ? matches : matches.slice(0, 6);
  return (
    <section className={expanded ? "panel match-panel expanded" : "panel match-panel"}>
      <div className="panel-heading">
        <div>
          <span className="eyebrow">LOCAL DATASET</span>
          <h2>{expanded ? "매치 처리 기록" : "최근 매치"}</h2>
        </div>
        <span className="record-count">{matches.length} records</span>
      </div>
      {visible.length === 0 ? (
        <div className="empty-state compact">
          <strong>저장된 매치가 없습니다</strong>
          <span>첫 동기화를 실행해 주세요.</span>
        </div>
      ) : (
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>시간</th>
                <th>맵</th>
                <th>모드</th>
                <th>상태</th>
                <th>매치 ID</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((match) => (
                <tr key={match.matchId} title={match.error ?? undefined}>
                  <td>{formatDate(match.createdAt)}</td>
                  <td>{match.mapName ? formatPubgMapName(match.mapName) : "Unknown"}</td>
                  <td>{match.gameMode ?? "—"}</td>
                  <td><Status status={match.status} /></td>
                  <td><code>{shortId(match.matchId)}</code></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function Status({ status }: { status: string }) {
  const label = status === "analyzed" ? "분석 완료" : status === "failed" ? "실패" : "처리 중";
  return <span className={`table-status ${status}`}>{label}</span>;
}

function formatDate(value: string | null): string {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("ko-KR", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function shortId(value: string): string {
  return value.length > 16 ? `${value.slice(0, 8)}…${value.slice(-5)}` : value;
}
