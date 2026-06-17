import { FormEvent, useState } from "react";

import { searchHybrid } from "../../lib/ipc";
import type { NoteRef, SearchHit } from "../../lib/types";

type SearchPanelProps = {
  onOpenNote: (note: NoteRef) => void;
};

function titleFromPath(path: string) {
  const parts = path.replace(/\\/g, "/").split("/");
  return (parts[parts.length - 1] ?? path).replace(/\.md$/i, "");
}

function viaLabel(via: string) {
  switch (via) {
    case "fts":
      return "FTS";
    case "vec":
      return "VEC";
    case "both":
      return "İKİSİ";
    default:
      return via.toUpperCase();
  }
}

export function SearchPanel({ onOpenNote }: SearchPanelProps) {
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [loading, setLoading] = useState(false);
  const [searched, setSearched] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
      setHits(await searchHybrid(trimmed, 10));
    } catch {
      setError("Arama tamamlanamadı.");
      setHits([]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <section className="task-panel search-panel" aria-labelledby="search-title">
      <header className="panel-header">
        <div>
          <p className="eyebrow">Arama</p>
          <h1 id="search-title">Hibrit arama</h1>
        </div>
      </header>

      <form className="search-form" onSubmit={submit}>
        <label className="field-label" htmlFor="hybrid-search">
          Sorgu
        </label>
        <div className="input-row">
          <input
            autoComplete="off"
            className="text-input"
            id="hybrid-search"
            onChange={(event) => setQuery(event.currentTarget.value)}
            placeholder="Notlarda ara"
            type="search"
            value={query}
          />
          <button className="button primary" disabled={loading} type="submit">
            {loading ? "Aranıyor" : "Ara"}
          </button>
        </div>
      </form>

      {error ? <p className="notice error">{error}</p> : null}

      <div className="result-list" aria-label="Arama sonuçları">
        {searched && !loading && hits.length === 0 ? (
          <p className="empty-state">Sonuç yok.</p>
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
