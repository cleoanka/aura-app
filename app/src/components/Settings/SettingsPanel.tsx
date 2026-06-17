import { useEffect, useState } from "react";

import { getSettings, setSettings } from "../../lib/ipc";
import type { CacheMode, DefaultMode, LaneSettings, Settings, ThemeMode } from "../../lib/types";

type SettingsForm = {
  theme: ThemeMode;
  defaultMode: DefaultMode;
  lanes: Required<LaneSettings>;
  consensusEnabled: boolean;
  cacheMode: CacheMode;
};

const defaultForm: SettingsForm = {
  theme: "dark",
  defaultMode: "ask",
  lanes: {
    fast: true,
    deep: true,
    lane0: false,
  },
  consensusEnabled: false,
  cacheMode: "read_write",
};

function normalize(settings: Settings | null): SettingsForm {
  return {
    theme: settings?.theme === "light" ? "light" : "dark",
    defaultMode: settings?.default_mode === "aura" ? "aura" : "ask",
    lanes: {
      fast: settings?.lanes?.fast ?? defaultForm.lanes.fast,
      deep: settings?.lanes?.deep ?? defaultForm.lanes.deep,
      lane0: settings?.lanes?.lane0 ?? defaultForm.lanes.lane0,
    },
    consensusEnabled: settings?.consensus_enabled ?? defaultForm.consensusEnabled,
    cacheMode: settings?.cache_mode ?? defaultForm.cacheMode,
  };
}

export function SettingsPanel() {
  const [baseSettings, setBaseSettings] = useState<Settings>({});
  const [form, setForm] = useState<SettingsForm>(defaultForm);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;

    void getSettings()
      .then((settings) => {
        if (!alive) {
          return;
        }

        const nextForm = normalize(settings);
        setBaseSettings(settings);
        setForm(nextForm);
        document.documentElement.dataset.theme = nextForm.theme;
      })
      .catch(() => {
        if (alive) {
          setError("Ayarlar alınamadı. Varsayılanlar gösteriliyor.");
        }
      })
      .finally(() => {
        if (alive) {
          setLoading(false);
        }
      });

    return () => {
      alive = false;
    };
  }, []);

  const updateTheme = (theme: ThemeMode) => {
    setForm((current) => ({ ...current, theme }));
    document.documentElement.dataset.theme = theme;
  };

  const updateLane = (lane: keyof LaneSettings, enabled: boolean) => {
    setForm((current) => ({
      ...current,
      lanes: {
        ...current.lanes,
        [lane]: enabled,
      },
    }));
  };

  const save = async () => {
    setSaving(true);
    setMessage(null);
    setError(null);

    const nextSettings: Settings = {
      ...baseSettings,
      theme: form.theme,
      default_mode: form.defaultMode,
      lanes: {
        ...(baseSettings.lanes ?? {}),
        ...form.lanes,
      },
      consensus_enabled: form.consensusEnabled,
      cache_mode: form.cacheMode,
    };

    try {
      await setSettings(nextSettings);
      setBaseSettings(nextSettings);
      setMessage("Ayarlar kaydedildi.");
    } catch {
      setError("Ayarlar kaydedilemedi.");
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className="task-panel settings-panel" aria-labelledby="settings-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">Ayarlar</p>
          <h1 id="settings-title">Çalışma tercihleri</h1>
        </div>
        <button className="button primary" disabled={loading || saving} onClick={save} type="button">
          {saving ? "Kaydediliyor" : "Kaydet"}
        </button>
      </header>

      {error ? <p className="notice error">{error}</p> : null}
      {message ? <p className="notice success">{message}</p> : null}

      <div className="settings-grid">
        <fieldset className="settings-group">
          <legend>Tema</legend>
          <label className="radio-row">
            <input
              checked={form.theme === "dark"}
              name="theme"
              onChange={() => updateTheme("dark")}
              type="radio"
            />
            <span>Koyu</span>
          </label>
          <label className="radio-row">
            <input
              checked={form.theme === "light"}
              name="theme"
              onChange={() => updateTheme("light")}
              type="radio"
            />
            <span>Açık</span>
          </label>
        </fieldset>

        <fieldset className="settings-group">
          <legend>Varsayılan mod</legend>
          <label className="radio-row">
            <input
              checked={form.defaultMode === "ask"}
              name="default-mode"
              onChange={() => setForm((current) => ({ ...current, defaultMode: "ask" }))}
              type="radio"
            />
            <span>ASK</span>
          </label>
          <label className="radio-row">
            <input
              checked={form.defaultMode === "aura"}
              name="default-mode"
              onChange={() => setForm((current) => ({ ...current, defaultMode: "aura" }))}
              type="radio"
            />
            <span>Aura</span>
          </label>
        </fieldset>

        <fieldset className="settings-group">
          <legend>Lane'ler</legend>
          <label className="toggle-row">
            <span>Fast</span>
            <input
              checked={form.lanes.fast}
              onChange={(event) => updateLane("fast", event.currentTarget.checked)}
              type="checkbox"
            />
          </label>
          <label className="toggle-row">
            <span>Deep</span>
            <input
              checked={form.lanes.deep}
              onChange={(event) => updateLane("deep", event.currentTarget.checked)}
              type="checkbox"
            />
          </label>
          <label className="toggle-row">
            <span>Lane 0</span>
            <input
              checked={form.lanes.lane0}
              onChange={(event) => updateLane("lane0", event.currentTarget.checked)}
              type="checkbox"
            />
          </label>
        </fieldset>

        <fieldset className="settings-group">
          <legend>AI çalıştırma</legend>
          <label className="toggle-row">
            <span>Konsensüs <small>3x maliyet</small></span>
            <input
              checked={form.consensusEnabled}
              onChange={(event) =>
                setForm((current) => ({
                  ...current,
                  consensusEnabled: event.currentTarget.checked,
                }))
              }
              type="checkbox"
            />
          </label>

          <label className="field-label" htmlFor="cache-mode">
            Önbellek
          </label>
          <select
            className="text-input"
            id="cache-mode"
            onChange={(event) =>
              setForm((current) => ({ ...current, cacheMode: event.currentTarget.value }))
            }
            value={form.cacheMode}
          >
            <option value="off">Kapalı</option>
            <option value="read">Oku</option>
            <option value="write">Yaz</option>
            <option value="read_write">Oku + yaz</option>
          </select>
        </fieldset>
      </div>
    </section>
  );
}
