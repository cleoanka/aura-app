import { useEffect, useState } from "react";

import { useI18n } from "../../i18n";
import { getSettings, setSettings } from "../../lib/ipc";
import type { CacheMode, DefaultMode, LaneSettings, Settings, ThemeMode } from "../../lib/types";

type SettingsForm = {
  theme: ThemeMode;
  defaultMode: DefaultMode;
  lanes: Required<LaneSettings>;
  consensusEnabled: boolean;
  cacheMode: CacheMode;
  semanticSearch: boolean;
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
  semanticSearch: false,
  cacheMode: "exact",
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
    semanticSearch: settings?.semantic_search ?? defaultForm.semanticSearch,
  };
}

export function SettingsPanel() {
  const { t } = useI18n();
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
          setError(t("common.error"));
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
  }, [t]);

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
      semantic_search: form.semanticSearch,
    };

    try {
      await setSettings(nextSettings);
      setBaseSettings(nextSettings);
      setMessage(t("settings.saved"));
      // Hep-mount panellere (Ask) ayarın değiştiğini bildir → anında yansısın.
      window.dispatchEvent(new CustomEvent("aura:settings-saved"));
    } catch {
      setError(t("common.error"));
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className="task-panel settings-panel" aria-labelledby="settings-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">{t("settings.title")}</p>
          <h1 id="settings-title">{t("settings.title")}</h1>
        </div>
        <button className="button primary" disabled={loading || saving} onClick={save} type="button">
          {saving ? t("common.loading") : t("settings.save")}
        </button>
      </header>

      {error ? <p className="notice error">{error}</p> : null}
      {message ? <p className="notice success">{message}</p> : null}

      <div className="settings-grid">
        <fieldset className="settings-group">
          <legend>{t("settings.theme")}</legend>
          <label className="radio-row">
            <input
              checked={form.theme === "dark"}
              name="theme"
              onChange={() => updateTheme("dark")}
              type="radio"
            />
            <span>{t("settings.theme.dark")}</span>
          </label>
          <label className="radio-row">
            <input
              checked={form.theme === "light"}
              name="theme"
              onChange={() => updateTheme("light")}
              type="radio"
            />
            <span>{t("settings.theme.light")}</span>
          </label>
        </fieldset>

        <fieldset className="settings-group">
          <legend>{t("settings.defaultMode")}</legend>
          <label className="radio-row">
            <input
              checked={form.defaultMode === "ask"}
              name="default-mode"
              onChange={() => setForm((current) => ({ ...current, defaultMode: "ask" }))}
              type="radio"
            />
            <span>{t("settings.mode.ask")}</span>
          </label>
          <label className="radio-row">
            <input
              checked={form.defaultMode === "aura"}
              name="default-mode"
              onChange={() => setForm((current) => ({ ...current, defaultMode: "aura" }))}
              type="radio"
            />
            <span>{t("settings.mode.aura")}</span>
          </label>
        </fieldset>

        <fieldset className="settings-group">
          <legend>{t("settings.lanes")}</legend>
          <label className="toggle-row">
            <span>{t("settings.lane.fast")}</span>
            <input
              checked={form.lanes.fast}
              onChange={(event) => updateLane("fast", event.currentTarget.checked)}
              type="checkbox"
            />
          </label>
          <label className="toggle-row">
            <span>{t("settings.lane.deep")}</span>
            <input
              checked={form.lanes.deep}
              onChange={(event) => updateLane("deep", event.currentTarget.checked)}
              type="checkbox"
            />
          </label>
          <label className="toggle-row">
            <span>{t("settings.lane.lane0")}</span>
            <input
              checked={form.lanes.lane0}
              onChange={(event) => updateLane("lane0", event.currentTarget.checked)}
              type="checkbox"
            />
          </label>
        </fieldset>

        <fieldset className="settings-group">
          <legend>{t("settings.localGen")}</legend>
          <label className="toggle-row">
            <span>{t("settings.consensus")} <small>{t("settings.consensusCostHint")}</small></span>
            <input
              checked={form.consensusEnabled}
              onChange={(event) => {
                const checked = event.currentTarget.checked;
                setForm((current) => ({ ...current, consensusEnabled: checked }));
              }}
              type="checkbox"
            />
          </label>

          <label className="toggle-row">
            <span>{t("settings.semanticSearch")} <small>{t("settings.semanticHint")}</small></span>
            <input
              checked={form.semanticSearch}
              onChange={(event) => {
                const checked = event.currentTarget.checked;
                setForm((current) => ({ ...current, semanticSearch: checked }));
              }}
              type="checkbox"
            />
          </label>

          <label className="field-label" htmlFor="cache-mode">
            {t("settings.cacheMode")}
          </label>
          <select
            className="text-input"
            id="cache-mode"
            onChange={(event) => {
              const value = event.currentTarget.value;
              setForm((current) => ({ ...current, cacheMode: value }));
            }}
            value={form.cacheMode}
          >
            <option value="off">{t("settings.cache.off")}</option>
            <option value="exact">{t("settings.cache.exact")}</option>
            <option value="semantic">{t("settings.cache.semantic")}</option>
          </select>
        </fieldset>
      </div>
    </section>
  );
}
