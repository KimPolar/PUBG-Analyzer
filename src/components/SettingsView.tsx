import { useEffect, useState, type FormEvent } from "react";
import type { AppSettings, SettingsResponse } from "../types/domain";

interface SettingsViewProps {
  value: SettingsResponse;
  saving: boolean;
  onSave: (settings: AppSettings, apiKey: string | null) => Promise<void>;
}

export function SettingsView({ value, saving, onSave }: SettingsViewProps) {
  const [settings, setSettings] = useState(value.settings);
  const [apiKey, setApiKey] = useState("");
  const [removeKey, setRemoveKey] = useState(false);

  useEffect(() => setSettings(value.settings), [value.settings]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    const keyUpdate = removeKey ? "" : apiKey.trim() ? apiKey.trim() : null;
    await onSave(settings, keyUpdate);
    setApiKey("");
    setRemoveKey(false);
  }

  function update<K extends keyof AppSettings>(key: K, next: AppSettings[K]) {
    setSettings((current) => ({ ...current, [key]: next }));
  }

  return (
    <div className="view-stack narrow-view">
      <header className="page-header">
        <div>
          <span className="eyebrow">APPLICATION SETTINGS</span>
          <h1>수집 환경 설정</h1>
          <p>계정과 분석 단위를 저장하면 이후에는 한 번의 클릭으로 새 매치를 동기화합니다.</p>
        </div>
      </header>

      <form className="settings-form" onSubmit={submit}>
        <section className="panel settings-section">
          <div className="section-title">
            <span>01</span>
            <div><h2>PUBG 계정</h2><p>공식 PUBG API에서 매치 목록을 조회합니다.</p></div>
          </div>
          <div className="form-grid two-columns">
            <Field label="플레이어 이름" hint="대소문자를 포함한 정확한 닉네임">
              <input
                required
                value={settings.username}
                onChange={(event) => update("username", event.target.value)}
                placeholder="PlayerName"
              />
            </Field>
            <Field label="플랫폼 샤드" hint="플레이 중인 플랫폼">
              <select value={settings.platform} onChange={(event) => update("platform", event.target.value)}>
                <option value="steam">Steam</option>
                <option value="kakao">Kakao</option>
                <option value="xbox">Xbox</option>
                <option value="psn">PlayStation</option>
              </select>
            </Field>
          </div>
        </section>

        <section className="panel settings-section">
          <div className="section-title">
            <span>02</span>
            <div><h2>API 인증과 속도</h2><p>API 키는 운영체제의 보안 자격 증명 저장소에만 저장됩니다.</p></div>
          </div>
          <div className="credential-state">
            <span className={value.apiKeyConfigured ? "credential-dot ready" : "credential-dot"} />
            <strong>{value.apiKeyConfigured ? "API 키 저장됨" : "API 키가 필요합니다"}</strong>
            <small>SQLite와 로그에는 기록하지 않습니다.</small>
          </div>
          <div className="form-grid two-columns">
            <Field label="PUBG API 키" hint={value.apiKeyConfigured ? "비워두면 현재 키 유지" : "Bearer 접두사 없이 입력"}>
              <input
                type="password"
                autoComplete="off"
                value={apiKey}
                disabled={removeKey}
                onChange={(event) => setApiKey(event.target.value)}
                placeholder={value.apiKeyConfigured ? "••••••••••••••••" : "API key"}
              />
            </Field>
            <Field label="요청 한도 (RPM)" hint="PUBG 기본 한도는 보통 10 RPM">
              <input
                type="number"
                min={1}
                max={600}
                value={settings.rpm}
                onChange={(event) => update("rpm", Number(event.target.value))}
              />
            </Field>
          </div>
          {value.apiKeyConfigured ? (
            <label className="check-line danger-check">
              <input type="checkbox" checked={removeKey} onChange={(event) => setRemoveKey(event.target.checked)} />
              저장된 API 키 제거
            </label>
          ) : null}
        </section>

        <section className="panel settings-section">
          <div className="section-title">
            <span>03</span>
            <div><h2>분석 해상도</h2><p>더 작은 셀과 버킷은 정밀하지만 처리량과 DB 크기가 증가합니다.</p></div>
          </div>
          <div className="form-grid three-columns">
            <Field label="동시 다운로드" hint="1–16 작업">
              <input
                type="number"
                min={1}
                max={16}
                value={settings.downloadConcurrency}
                onChange={(event) => update("downloadConcurrency", Number(event.target.value))}
              />
            </Field>
            <Field label="공간 셀" hint="10–1000m">
              <div className="unit-input">
                <input
                  type="number"
                  min={10}
                  max={1000}
                  value={settings.cellSizeM}
                  onChange={(event) => update("cellSizeM", Number(event.target.value))}
                />
                <span>m</span>
              </div>
            </Field>
            <Field label="시간 버킷" hint="1–60초">
              <div className="unit-input">
                <input
                  type="number"
                  min={1}
                  max={60}
                  value={settings.bucketSeconds}
                  onChange={(event) => update("bucketSeconds", Number(event.target.value))}
                />
                <span>sec</span>
              </div>
            </Field>
          </div>
          <label className="check-line">
            <input
              type="checkbox"
              checked={settings.keepRawTelemetry}
              onChange={(event) => update("keepRawTelemetry", event.target.checked)}
            />
            분석 후 원본 텔레메트리를 gzip으로 보관
          </label>
        </section>

        <div className="form-actions">
          <span>설정 변경은 다음 수집부터 적용됩니다.</span>
          <button className="button primary" type="submit" disabled={saving}>
            {saving ? "저장 중…" : "설정 저장"}
          </button>
        </div>
      </form>
    </div>
  );
}

function Field({ label, hint, children }: { label: string; hint: string; children: React.ReactNode }) {
  return (
    <label className="form-field">
      <span>{label}</span>
      {children}
      <small>{hint}</small>
    </label>
  );
}
