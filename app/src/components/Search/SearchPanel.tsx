import { FormEvent, useState } from "react";

import { useI18n } from "../../i18n";
import { searchHybrid } from "../../lib/ipc";
import type { NoteRef, SearchHit } from "../../lib/types";

type SearchPanelProps = {
  onOpenNote: (note: NoteRef) => void;
};

function titleFromPath(path: string) {
  const parts = path.replace(/\\/g, "/").split("/");
  return (parts[parts.length - 1] ?? path).replace(/\.md$/i, "");
}

export function SearchPanel({ onOpenNote }: SearchPanelProps) {
  const { t } = useI18n();

  const viaLabel = (via: string) => {
    switch (via) {
      case "fts":
        return t("search.via.fts");
      case "vec":
        return t("search.via.vec");
      case "both":
        return t("search.via.both");
      default:
        return via.toUpperCase();
    }
  };

  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [loading, setLoading] = useState(false);
  const [searched, setSearched] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [elapsedMs, setElapsedMs] = useState<number | null>(null);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmed = query.trim();

    if (!trimmed) {
      setHits([]);
      setSearched(false);
      return;
    }

    setLoading(true);
    setError(null);
    setSearched(true);

    try {
      const started = performance.now();
      const results = await searchHybrid(trimmed, 10);
      setElapsedMs(Math.round(performance.now() - started));
      setHits(results);
    } catch {
      setError(t("ask.error"));
      setHits([]);
      setElapsedMs(null);
    } finally {
      setLoading(false);
    }
  };

  return (
    <section className="task-panel search-panel" aria-labelledby="search-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">{t("nav.search")}</p>
          <h1 id="search-title">{t("search.via.both")}</h1>
        </div>
      </header>

      <form className="search-form" onSubmit={submit}>
        <label className="field-label" htmlFor="hybrid-search">
          {t("nav.search")}
        </label>
        <div className="input-row">
          <input
            autoComplete="off"
            className="text-input"
            id="hybrid-search"
            onChange={(event) => setQuery(event.currentTarget.value)}
            placeholder={t("search.placeholder")}
            type="search"
            value={query}
          />
          <button className="button primary" disabled={loading} type="submit">
            {loading ? t("common.loading") : t("search.button")}
          </button>
        </div>
      </form>

      {error ? <p className="notice error">{error}</p> : null}

      <div className="result-list" aria-label={t("nav.search")}>
        {searched && !loading && hits.length === 0 ? (
          <p className="empty-state">{t("search.noResults")}</p>
        ) : null}

        {!loading && hits.length > 0 ? (
          <p className="result-summary">
            {hits.length} {t("search.results")}
            {elapsedMs !== null ? ` · ${elapsedMs} ms` : ""}
          </p>
        ) : null}

        {hits.map((hit) => {
          const title = titleFromPath(hit.note_path);

          return (
            <button
              aria-label={`${title} sonucunu aç`}
              className="result-item"
              key={`${hit.note_path}:${hit.heading_path}:${hit.snippet}`}
              onClick={() => onOpenNote({ path: hit.note_path, title })}
              type="button"
            >
              <span className="result-topline">
                <span className="result-title">{title}</span>
                <span className={`badge via-${hit.via}`}>{viaLabel(hit.via)}</span>
              </span>
              {hit.heading_path ? (
                <span className="result-heading">{hit.heading_path}</span>
              ) : null}
              <span className="result-snippet">{hit.snippet}</span>
            </button>
          );
        })}
      </div>
    </section>
  );
}
