import { useCallback, useEffect, useMemo, useState } from "react";

import { useI18n } from "../../i18n";
import { indexVault, listNotes, pickVaultFolder } from "../../lib/ipc";
import type { IndexStats, NoteRef } from "../../lib/types";

type VaultExplorerProps = {
  activePath: string | null;
  onNotesChange: (count: number) => void;
  onOpenNote: (note: NoteRef) => void;
};

type NoteGroup = {
  name: string;
  notes: NoteRef[];
};

function folderName(path: string) {
  const normalized = path.replace(/\\/g, "/");
  const parts = normalized.split("/");

  if (parts.length <= 1) {
    return "Kök";
  }

  const folder = parts.slice(0, -1).join("/");
  return folder || "Kök";
}

function sortNotes(notes: NoteRef[]) {
  return [...notes].sort((left, right) => {
    const titleOrder = left.title.localeCompare(right.title, "tr");
    return titleOrder === 0 ? left.path.localeCompare(right.path, "tr") : titleOrder;
  });
}

export function VaultExplorer({
  activePath,
  onNotesChange,
  onOpenNote,
}: VaultExplorerProps) {
  const { t } = useI18n();
  const [notes, setNotes] = useState<NoteRef[]>([]);
  const [loading, setLoading] = useState(true);
  const [indexing, setIndexing] = useState(false);
  const [indexStats, setIndexStats] = useState<IndexStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refreshNotes = useCallback(async () => {
    setError(null);
    setLoading(true);

    try {
      const nextNotes = sortNotes(await listNotes());
      setNotes(nextNotes);
      onNotesChange(nextNotes.length);
    } catch {
      setError(t("common.error"));
      setNotes([]);
      onNotesChange(0);
    } finally {
      setLoading(false);
    }
  }, [onNotesChange, t]);

  useEffect(() => {
    void refreshNotes();
  }, [refreshNotes]);

  const groups = useMemo<NoteGroup[]>(() => {
    const byFolder = new Map<string, NoteRef[]>();

    for (const note of notes) {
      const key = folderName(note.path);
      byFolder.set(key, [...(byFolder.get(key) ?? []), note]);
    }

    return [...byFolder.entries()]
      .sort(([left], [right]) => left.localeCompare(right, "tr"))
      .map(([name, groupNotes]) => ({ name, notes: sortNotes(groupNotes) }));
  }, [notes]);

  const selectVault = async () => {
    setError(null);
    setIndexing(true);
    setIndexStats(null);

    try {
      const path = await pickVaultFolder();

      if (!path) {
        return;
      }

      const stats = await indexVault(path);
      setIndexStats(stats);
      onNotesChange(stats.notes);
      await refreshNotes();
    } catch {
      setError(t("common.error"));
    } finally {
      setIndexing(false);
    }
  };

  return (
    <aside className="vault-panel" aria-label={t("nav.workspace")}>
      <div className="panel-header compact">
        <div>
          <p className="eyebrow">{t("nav.workspace")}</p>
          <h2>{t("workspace.title")}</h2>
        </div>
        <button
          aria-label={t("workspace.openFolder")}
          className="button primary"
          disabled={indexing}
          onClick={selectVault}
          type="button"
        >
          {indexing ? t("common.loading") : t("workspace.openFolder")}
        </button>
      </div>

      {indexStats ? (
        <div className="index-stats" aria-label={t("status.indexed")}>
          <span>{t("workspace.notesCount", { count: indexStats.notes })}</span>
          <span>
            {indexStats.chunks} · {indexStats.skipped}
            {indexStats.pruned ? ` · −${indexStats.pruned}` : ""}
            {typeof indexStats.elapsed_ms === "number"
              ? ` · ${indexStats.elapsed_ms} ms`
              : ""}
          </span>
        </div>
      ) : null}

      {error ? <p className="notice error">{error}</p> : null}
      {loading ? <p className="notice">{t("common.loading")}</p> : null}

      <div className="note-tree" aria-label={t("workspace.title")}>
        {!loading && groups.length === 0 ? (
          <p className="empty-state">{t("workspace.noNotes")}</p>
        ) : null}

        {groups.map((group) => (
          <section className="note-group" key={group.name}>
            <h3>{group.name}</h3>
            {group.notes.map((note) => (
              <button
                aria-label={`${note.title} notunu aç`}
                className={`note-row ${activePath === note.path ? "is-active" : ""}`}
                key={note.path}
                onClick={() => onOpenNote(note)}
                title={note.path}
                type="button"
              >
                <span className="note-title">{note.title || note.path}</span>
              </button>
            ))}
          </section>
        ))}
      </div>
    </aside>
  );
}
