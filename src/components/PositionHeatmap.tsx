import { useDeferredValue, useId, useMemo, useState } from "react";
import { formatPubgMapName, getPubgMapDefinition, getPubgMapImageUrl } from "../lib/pubg-maps";
import type { DatasetAnalysis, DatasetCell } from "../types/domain";

interface PositionHeatmapProps {
  dataset: DatasetAnalysis | null;
}

interface Viewport {
  x: number;
  y: number;
  size: number;
}

const EMPTY_CELLS: DatasetCell[] = [];
const MIN_CELL_SIZE_M = 10;
const MAX_CELL_SIZE_M = 1_000;
const MAX_ZOOM = 4;

export function PositionHeatmap({ dataset }: PositionHeatmapProps) {
  const maps = dataset?.maps ?? [];
  const [requestedMap, setRequestedMap] = useState("");
  const [requestedPhase, setRequestedPhase] = useState<number | null>(null);
  const [selectedCell, setSelectedCell] = useState<DatasetCell | null>(null);
  const [zoom, setZoom] = useState(1);
  const [showMap, setShowMap] = useState(true);
  const [showGrid, setShowGrid] = useState(true);
  const svgId = useId().replaceAll(":", "");

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
  const mapDefinition = activeMap ? getPubgMapDefinition(activeMap.mapName) : undefined;
  const cellSizeM = useMemo(() => inferCellSizeM(deferredCells), [deferredCells]);
  const worldSizeM = mapDefinition?.worldSizeM ?? inferFallbackWorldSizeM(deferredCells, cellSizeM);
  const renderedCells = useMemo(
    () => deferredCells.filter((cell) => isInsideMap(cell, worldSizeM, cellSizeM)),
    [cellSizeM, deferredCells, worldSizeM],
  );
  const highlighted = selectedCell
    ? renderedCells.find((cell) => cell.phaseCellId === selectedCell.phaseCellId) ?? renderedCells[0] ?? null
    : renderedCells[0] ?? null;
  const viewport = useMemo(
    () => calculateViewport(worldSizeM, zoom, highlighted),
    [highlighted, worldSizeM, zoom],
  );

  const selectMap = (mapName: string) => {
    setRequestedMap(mapName);
    setRequestedPhase(null);
    setSelectedCell(null);
    setZoom(1);
  };

  const selectPhase = (phase: number) => {
    setRequestedPhase(phase);
    setSelectedCell(null);
    setZoom(1);
  };

  return (
    <section className="panel heatmap-panel">
      <div className="panel-heading heatmap-heading">
        <div>
          <span className="eyebrow">HISTORICAL POSITION VALUE</span>
          <h2>지도 기반 포지션 가치</h2>
        </div>
        <div className="heatmap-filters">
          <label>
            <span className="sr-only">맵</span>
            <select value={activeMap?.mapName ?? ""} onChange={(event) => selectMap(event.target.value)}>
              {maps.map((map) => (
                <option value={map.mapName} key={map.mapName}>
                  {formatPubgMapName(map.mapName)} · {map.matchCount} matches
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
                onClick={() => selectPhase(phase)}
              >
                P{phase}
              </button>
            ))}
          </div>
        </div>
      </div>

      {renderedCells.length === 0 ? (
        <div className="empty-state">
          <strong>표시할 분석 셀이 없습니다</strong>
          <span>매치를 수집하면 페이즈별 포지션 가치가 지도 위에 표시됩니다.</span>
        </div>
      ) : (
        <div className="heatmap-layout">
          <div className="map-canvas">
            <div className="map-toolbar" aria-label="지도 레이어 및 확대 제어">
              <button
                type="button"
                className={showMap ? "active" : ""}
                aria-pressed={showMap}
                onClick={() => setShowMap((value) => !value)}
              >
                지도
              </button>
              <button
                type="button"
                className={showGrid ? "active" : ""}
                aria-pressed={showGrid}
                onClick={() => setShowGrid((value) => !value)}
              >
                1km 격자
              </button>
              <span className="toolbar-divider" />
              <button
                type="button"
                aria-label="축소"
                disabled={zoom <= 1}
                onClick={() => setZoom((value) => Math.max(1, value - 1))}
              >
                −
              </button>
              <output aria-label="현재 확대 배율">{zoom}×</output>
              <button
                type="button"
                aria-label="확대"
                disabled={zoom >= MAX_ZOOM}
                onClick={() => setZoom((value) => Math.min(MAX_ZOOM, value + 1))}
              >
                +
              </button>
            </div>

            <svg
              viewBox={`${viewport.x} ${viewport.y} ${viewport.size} ${viewport.size}`}
              role="img"
              aria-label={`${formatPubgMapName(activeMap?.mapName ?? "")} 페이즈 ${activePhase} 지도 기반 포지션 가치`}
              preserveAspectRatio="xMidYMid meet"
            >
              <title>{formatPubgMapName(activeMap?.mapName ?? "")} P{activePhase} 포지션 가치</title>
              <defs>
                <clipPath id={`${svgId}-clip`}>
                  <rect width={worldSizeM} height={worldSizeM} />
                </clipPath>
                <pattern id={`${svgId}-grid`} width="1000" height="1000" patternUnits="userSpaceOnUse">
                  <path d="M 1000 0 L 0 0 0 1000" className="map-kilometer-line" />
                </pattern>
              </defs>

              <g clipPath={`url(#${svgId}-clip)`}>
                <rect width={worldSizeM} height={worldSizeM} className="map-fallback" />
                {showMap && mapDefinition ? (
                  <image
                    href={getPubgMapImageUrl(mapDefinition)}
                    x="0"
                    y="0"
                    width={worldSizeM}
                    height={worldSizeM}
                    preserveAspectRatio="none"
                    className="map-background"
                  />
                ) : null}
                <rect width={worldSizeM} height={worldSizeM} className="map-tone" />
                {showGrid ? <rect width={worldSizeM} height={worldSizeM} fill={`url(#${svgId}-grid)`} /> : null}
                {renderedCells.map((cell) => {
                  const padding = cellSizeM * 0.05;
                  const active = highlighted?.phaseCellId === cell.phaseCellId;
                  return (
                    <rect
                      className={active ? "heat-cell selected" : "heat-cell"}
                      key={cell.phaseCellId}
                      x={cell.centerXM - cellSizeM / 2 + padding}
                      y={cell.centerYM - cellSizeM / 2 + padding}
                      width={cellSizeM - padding * 2}
                      height={cellSizeM - padding * 2}
                      rx={cellSizeM * 0.08}
                      fill={scoreColor(cell.historicalValueScore)}
                      fillOpacity={0.2 + cell.scoreConfidence * 0.58}
                      vectorEffect="non-scaling-stroke"
                      tabIndex={0}
                      role="button"
                      aria-label={`가치 ${cell.historicalValueScore.toFixed(1)}점, 좌표 ${Math.round(cell.centerXM)}m ${Math.round(cell.centerYM)}m`}
                      onFocus={() => setSelectedCell(cell)}
                      onClick={() => setSelectedCell(cell)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === " ") setSelectedCell(cell);
                      }}
                    >
                      <title>
                        {cell.historicalValueScore.toFixed(1)}점 · X {Math.round(cell.centerXM)}m · Y {Math.round(cell.centerYM)}m
                      </title>
                    </rect>
                  );
                })}
              </g>
            </svg>

            <div className="map-source">
              <strong>{formatPubgMapName(activeMap?.mapName ?? "")}</strong>
              <span>
                {(worldSizeM / 1000).toFixed(2)} × {(worldSizeM / 1000).toFixed(2)} km · {Math.round(cellSizeM)}m cells
              </span>
              {!mapDefinition ? <em>지도 자산 없음</em> : null}
            </div>
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
                <small>X {Math.round(highlighted.centerXM)}m · Y {Math.round(highlighted.centerYM)}m</small>
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

function inferCellSizeM(cells: DatasetCell[]): number {
  const candidates: number[] = [];
  for (const cell of cells) {
    const xDivisor = cell.cellX + 0.5;
    const yDivisor = cell.cellY + 0.5;
    if (xDivisor > 0) candidates.push(cell.centerXM / xDivisor);
    if (yDivisor > 0) candidates.push(cell.centerYM / yDivisor);
  }
  const valid = candidates.filter(
    (value) => Number.isFinite(value) && value >= MIN_CELL_SIZE_M && value <= MAX_CELL_SIZE_M,
  );
  if (valid.length === 0) return 100;
  valid.sort((left, right) => left - right);
  return valid[Math.floor(valid.length / 2)] ?? 100;
}

function inferFallbackWorldSizeM(cells: DatasetCell[], cellSizeM: number): number {
  let maxCoordinate = cellSizeM;
  for (const cell of cells) {
    maxCoordinate = Math.max(maxCoordinate, cell.centerXM + cellSizeM / 2, cell.centerYM + cellSizeM / 2);
  }
  return Math.max(1_000, Math.ceil(maxCoordinate / 1_000) * 1_000);
}

function isInsideMap(cell: DatasetCell, worldSizeM: number, cellSizeM: number): boolean {
  const halfCell = cellSizeM / 2;
  return cell.centerXM + halfCell > 0
    && cell.centerYM + halfCell > 0
    && cell.centerXM - halfCell < worldSizeM
    && cell.centerYM - halfCell < worldSizeM;
}

function calculateViewport(worldSizeM: number, zoom: number, cell: DatasetCell | null): Viewport {
  const size = worldSizeM / zoom;
  const focusX = cell?.centerXM ?? worldSizeM / 2;
  const focusY = cell?.centerYM ?? worldSizeM / 2;
  return {
    x: clamp(focusX - size / 2, 0, worldSizeM - size),
    y: clamp(focusY - size / 2, 0, worldSizeM - size),
    size,
  };
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}

function scoreColor(score: number): string {
  const normalized = Math.max(0, Math.min(1, (score - 25) / 70));
  const hue = 18 + normalized * 137;
  return `hsl(${hue} 82% 54%)`;
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
