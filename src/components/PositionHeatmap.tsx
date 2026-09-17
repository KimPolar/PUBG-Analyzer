import { useDeferredValue, useMemo, useState } from "react";
import type { DatasetAnalysis, DatasetCell } from "../types/domain";

interface PositionHeatmapProps {
  dataset: DatasetAnalysis | null;
}

const EMPTY_CELLS: DatasetCell[] = [];

export function PositionHeatmap({ dataset }: PositionHeatmapProps) {
  const maps = dataset?.maps ?? [];
  const [requestedMap, setRequestedMap] = useState("");
  const [requestedPhase, setRequestedPhase] = useState<number | null>(null);
  const [selectedCell, setSelectedCell] = useState<DatasetCell | null>(null);
  const activeMap = maps.find((map) => map.mapName === requestedMap) ?? maps[0];
  const phases = useMemo(
    () => [...new Set(activeMap?.cells.map((cell) => cell.phase) ?? [])].sort((a, b) => a - b),
    [activeMap],
  );
  const activePhase = requestedPhase !== null && phases.includes(requestedPhase) ? requestedPhase : phases[0];
  const phaseCells = useMemo(
    () => activeMap?.cells.filter((cell) => cell.phase === activePhase) ?? EMPTY_CELLS,
    [activeMap, activePhase],
  );
  const deferredCells = useDeferredValue(phaseCells);
  const bounds = useMemo(() => cellBounds(deferredCells), [deferredCells]);
  const highlighted = selectedCell
    ? deferredCells.find((cell) => cell.phaseCellId === selectedCell.phaseCellId) ?? deferredCells[0] ?? null
    : deferredCells[0] ?? null;

  return (
    <section className="panel heatmap-panel">
      <div className="panel-heading heatmap-heading">
        <div>
          <span className="eyebrow">HISTORICAL POSITION VALUE</span>
          <h2>포지션 가치 히트맵</h2>
        </div>
        <div className="heatmap-filters">
          <label>
            <span className="sr-only">맵</span>
            <select value={activeMap?.mapName ?? ""} onChange={(event) => setRequestedMap(event.target.value)}>
              {maps.map((map) => (
                <option value={map.mapName} key={map.mapName}>
                  {cleanMapName(map.mapName)} · {map.matchCount} matches
                </option>
              ))}
            </select>
          </label>
          <div className="phase-tabs" aria-label="페이즈 선택">
            {phases.map((phase) => (
              <button
                className={phase === activePhase ? "active" : ""}
                key={phase}
                type="button"
                onClick={() => {
                  setRequestedPhase(phase);
                  setSelectedCell(null);
                }}
              >
                P{phase}
              </button>
            ))}
          </div>
        </div>
      </div>

      {deferredCells.length === 0 || !bounds ? (
        <div className="empty-state">
          <strong>표시할 분석 셀이 없습니다</strong>
          <span>매치를 수집하면 페이즈별 포지션 가치가 이곳에 표시됩니다.</span>
        </div>
      ) : (
        <div className="heatmap-layout">
          <div className="map-canvas">
            <div className="map-grid" aria-hidden="true" />
            <svg
              viewBox={`0 0 ${bounds.width} ${bounds.height}`}
              role="img"
              aria-label={`${cleanMapName(activeMap?.mapName ?? "")} 페이즈 ${activePhase} 포지션 가치 히트맵`}
              preserveAspectRatio="xMidYMid meet"
            >
              {deferredCells.map((cell) => {
                const x = cell.cellX - bounds.minX;
                const y = bounds.maxY - cell.cellY;
                const active = highlighted?.phaseCellId === cell.phaseCellId;
                return (
                  <rect
                    className={active ? "heat-cell selected" : "heat-cell"}
                    key={cell.phaseCellId}
                    x={x + 0.05}
                    y={y + 0.05}
                    width={0.9}
                    height={0.9}
                    rx={0.11}
                    fill={scoreColor(cell.historicalValueScore)}
                    fillOpacity={0.32 + cell.scoreConfidence * 0.68}
                    tabIndex={0}
                    role="button"
                    aria-label={`가치 ${cell.historicalValueScore.toFixed(1)}점, 셀 ${cell.cellId}`}
                    onMouseEnter={() => setSelectedCell(cell)}
                    onFocus={() => setSelectedCell(cell)}
                    onClick={() => setSelectedCell(cell)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" || event.key === " ") setSelectedCell(cell);
                    }}
                  />
                );
              })}
            </svg>
            <div className="heat-legend">
              <span>낮음</span>
              <i />
              <span>높음</span>
            </div>
          </div>

          <aside className="cell-inspector">
            <span className="eyebrow">SELECTED CELL</span>
            {highlighted ? (
              <>
                <div className="cell-score">
                  <strong>{highlighted.historicalValueScore.toFixed(1)}</strong>
                  <span>POSITION VALUE</span>
                </div>
                <dl>
                  <Metric label="120초 생존" value={formatRate(highlighted.survival120s)} />
                  <Metric label="다음 원 잔류" value={formatRate(highlighted.nextZoneRetention)} />
                  <Metric label="평균 점유" value={formatSeconds(highlighted.meanHoldSeconds)} />
                  <Metric label="300m 적 팀" value={highlighted.meanEnemyTeams300m.toFixed(2)} />
                  <Metric label="교전 손익" value={signed(highlighted.damageBalance)} />
                  <Metric label="관측 매치" value={`${highlighted.matchCount}`} />
                </dl>
                <div className="confidence-line">
                  <span>신뢰도</span>
                  <div><i style={{ width: `${highlighted.scoreConfidence * 100}%` }} /></div>
                  <strong>{Math.round(highlighted.scoreConfidence * 100)}%</strong>
                </div>
                <small>좌표 {Math.round(highlighted.centerXM)}m, {Math.round(highlighted.centerYM)}m</small>
              </>
            ) : null}
          </aside>
        </div>
      )}
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

function cellBounds(cells: DatasetCell[]) {
  if (cells.length === 0) return null;
  let minX = cells[0]!.cellX;
  let maxX = minX;
  let minY = cells[0]!.cellY;
  let maxY = minY;
  for (const cell of cells) {
    minX = Math.min(minX, cell.cellX);
    maxX = Math.max(maxX, cell.cellX);
    minY = Math.min(minY, cell.cellY);
    maxY = Math.max(maxY, cell.cellY);
  }
  return { minX, maxX, minY, maxY, width: maxX - minX + 1, height: maxY - minY + 1 };
}

function scoreColor(score: number): string {
  const normalized = Math.max(0, Math.min(1, (score - 25) / 70));
  const hue = 18 + normalized * 137;
  return `hsl(${hue} 76% 54%)`;
}

function formatRate(value: number | null): string {
  return value === null ? "—" : `${Math.round(value * 100)}%`;
}

function formatSeconds(value: number | null): string {
  return value === null ? "—" : `${Math.round(value)}초`;
}

function signed(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(1)}`;
}

function cleanMapName(value: string): string {
  return value.replace(/_Main$/u, "");
}
