# PUBG Position Analyzer

PUBG 원본 텔레메트리에서 **다음 원 좌표 자체가 아니라, 각 위치를 선점했을 때의 역사적 가치**를 분석하기 위한 프로젝트입니다.

현재 버전은 모델 학습 전 단계인 신뢰 가능한 feature extraction MVP입니다. 한 경기 또는 여러 경기에서 다음 정보를 추출할 수 있는 기반을 제공합니다.

- `LogPhaseChange`와 `LogGameStatePeriodic` 기반 자기장 단계
- 플레이어 좌표를 합친 팀 중심·팀 분산·차량 탑승 상태
- 현재 원 기준 거리와 정규화 좌표
- 다음 원 포함 여부와 진입 필요 거리
- 100m·300m·500m 주변 적 팀 밀도
- 30초·60초·120초 생존 label
- 팀별 실제 이동 구간
- 피해·다운·킬·사망의 공간 집계
- 관측 `z` 하위 분위수를 사용한 고도 proxy
- 페이즈별 100m 셀의 기술 통계와 보수적으로 축소한 역사적 위치 점수

## 설치

Python 3.11 이상이 필요합니다.

```bash
python -m venv .venv
source .venv/bin/activate
python -m pip install -e ".[dev]"
```

Windows PowerShell에서는 가상환경 활성화 명령이 다음과 같습니다.

```powershell
.venv\Scripts\Activate.ps1
```

## CLI 실행

로컬 JSON과 gzip 압축 텔레메트리를 자동 판별합니다. PUBG CDN URL도 직접 입력할 수 있습니다.

```bash
pubg-analyzer analyze \
  "https://telemetry-cdn.pubg.com/bluehole-pubg/steam/2026/09/17/01/57/2773ed27-b23b-11f1-9ce7-960e258f6262-telemetry.json" \
  --output analysis.json
```

팀 단위 row와 이동 구간까지 확인하려면 다음 옵션을 사용합니다.

```bash
pubg-analyzer analyze telemetry.json \
  --include-snapshots \
  --include-movement-segments \
  --output analysis.json
```

기본 공간 셀은 `100m × 100m`, 시간 bucket은 10초입니다.

```bash
pubg-analyzer analyze telemetry.json \
  --cell-size-m 100 \
  --bucket-seconds 10 \
  --output analysis.json
```

## API 실행

```bash
pubg-analyzer serve --host 0.0.0.0 --port 8000
```

개발 서버가 뜨면 `http://localhost:8000/docs`에서 요청을 시험할 수 있습니다.

```bash
curl -X POST http://localhost:8000/v1/analyze/url \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://telemetry-cdn.pubg.com/bluehole-pubg/steam/2026/09/17/01/57/2773ed27-b23b-11f1-9ce7-960e258f6262-telemetry.json",
    "cell_size_m": 100,
    "bucket_seconds": 10
  }'
```

API의 URL 분석은 SSRF 방지를 위해 `telemetry-cdn.pubg.com`만 허용합니다. 로컬 파일은 CLI에서 분석합니다.

## 결과 구조

```text
match                 경기와 자기장 설정 메타데이터
summary               이벤트·팀·스냅샷·셀 개수
event_counts          이벤트 종류별 개수
phase_timeline        페이즈 전환 시각
circles               페이즈별 목표 자기장
position_cells        페이즈·공간 셀별 분석 결과
movement_summary      팀 이동 요약
team_snapshots        --include-snapshots 사용 시 포함
movement_segments     --include-movement-segments 사용 시 포함
```

PUBG 텔레메트리의 `safetyZonePosition`은 블루존이 닫히는 동안 계속 움직입니다. 이 프로젝트는 페이즈별 목표 원을 구할 때 고정되어 있는 `poisonGasWarningPosition`과 `poisonGasWarningRadius`를 사용하고, 최초 원만 초기 `safetyZonePosition`을 사용합니다.

## 점수 해석

`historical_value_score`는 현재 입력된 관측 데이터 안에서 다음 항목을 합친 기술 통계입니다.

- 120초 생존
- 다음 원 잔류
- 자리 유지 시간
- 다음 원 진입 부담
- 피해 교환
- 주변 적 팀 경쟁도

표본이 적은 셀은 50점으로 축소하고 `score_confidence`를 함께 제공합니다. 이 점수는 아직 인과효과나 실시간 추천값이 아닙니다. 여러 경기 적재, 팀 전력·도착 시각·생존 인원 등의 선택 편향 보정, walk-forward 검증을 추가한 뒤 Position Value 모델의 학습 target/feature로 사용해야 합니다.

## 테스트

```bash
ruff check .
pytest
```

