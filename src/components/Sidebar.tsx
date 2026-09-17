import type { ViewName } from "../types/domain";

interface SidebarProps {
  activeView: ViewName;
  playerName: string | null;
  platform: string | null;
  onNavigate: (view: ViewName) => void;
}

const navigation: Array<{ view: ViewName; label: string; glyph: string }> = [
  { view: "dashboard", label: "포지션 분석", glyph: "⌖" },
  { view: "matches", label: "매치 기록", glyph: "▤" },
  { view: "training", label: "모델 학습", glyph: "◇" },
  { view: "settings", label: "설정", glyph: "⚙" },
];

export function Sidebar({ activeView, playerName, platform, onNavigate }: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark" aria-hidden="true">
          PA
        </div>
        <div>
          <strong>PUBG ANALYZER</strong>
          <span>POSITION INTELLIGENCE</span>
        </div>
      </div>

      <nav className="nav-list" aria-label="주 메뉴">
        {navigation.map((item) => (
          <button
            className={activeView === item.view ? "nav-item active" : "nav-item"}
            key={item.view}
            type="button"
            onClick={() => onNavigate(item.view)}
          >
            <span className="nav-glyph" aria-hidden="true">
              {item.glyph}
            </span>
            {item.label}
          </button>
        ))}
      </nav>

      <div className="sidebar-profile">
        <span className="eyebrow">ANALYSIS TARGET</span>
        <strong>{playerName ?? "플레이어 미설정"}</strong>
        <span>{platform?.toUpperCase() ?? "설정에서 계정을 등록하세요"}</span>
      </div>
      <div className="sidebar-build">RUST CORE · v0.2.0</div>
    </aside>
  );
}
