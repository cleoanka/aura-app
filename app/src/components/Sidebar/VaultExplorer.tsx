import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useI18n } from "../../i18n";
import {
  forgetWorkspace,
  getWorkspace,
  indexVault,
  listNotes,
  pickVaultFolder,
  setActiveWorkspace,
} from "../../lib/ipc";
import type { IndexStats, NoteRef, WorkspaceInfo } from "../../lib/types";

type VaultExplorerProps = {
  activePath: string | null;
  /** Arttığında liste + workspace bilgisi yeniden çekilir (remount değil → scroll korunur). */
  refreshToken: number;
  onNotesChange: (count: number) => void;
  onOpenNote: (note: NoteRef) => void;
  /** Aktif workspace değişti (seç/geç/unut) → üst katman seçim/görünümleri sıfırlar. */
  onWorkspaceChange: () => void;
};

type NoteGroup = {
  name: string;
  notes: NoteRef[];
};

// Kök grubu için dil-bağımsız sentinel: gruplama anahtarı stabil kalır, render'da çevrilir.
const ROOT_GROUP = "";

function folderName(path: string) {
  const normalized = path.replace(/\\/g, "/");
  const parts = normalized.split("/");

  if (parts.length <= 1) {
    return ROOT_GROUP;
  }

  return parts.slice(0, -1).join("/") || ROOT_GROUP;
}

function baseName(path: string) {
  const normalized = path.replace(/\\/g, "/").replace(/\/+$/, "");
  const parts = normalized.split("/");
  return parts[parts.length - 1] || normalized;
}

function sortNotes(notes: NoteRef[]) {
  return [...notes].sort((left, right) => {
    const titleOrder = left.title.localeCompare(right.title, "tr");
    return titleOrder === 0 ? left.path.localeCompare(right.path, "tr") : titleOrder;
  });
}

export function VaultExplorer({
  activePath,
  refreshToken,
  onNotesChange,
  onOpenNote,
  onWorkspaceChange,
}: VaultExplorerProps) {
  const { t } = useI18n();
  const [notes, setNotes] = useState<NoteRef[]>([]);
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [indexing, setIndexing] = useState(false);
  const [indexStats, setIndexStats] = useState<IndexStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);

  const refresh = useCallback(async () => {
    setError(null);
    setLoading(true);

    try {
      const [nextNotes, nextWorkspace] = await Promise.all([
        listNotes().then(sortNotes),
        getWorkspace(),
      ]);
      setNotes(nextNotes);
      setWorkspace(nextWorkspace);
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
    void refresh();
  }, [refresh, refreshToken]);

  // Menü dışına tıklayınca kapan (dependency'siz, tek global listener).
  useEffect(() => {
    if (!menuOpen) {
      return;
    }
    const close = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    };
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [menuOpen]);

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

  const activateAndIndex = useCallback(
    async (path: string) => {
      setError(null);
      setIndexing(true);
      setIndexStats(null);
      setMenuOpen(false);

      try {
        const stats = await indexVault(path);
        setIndexStats(stats);
        onWorkspaceChange();
      } catch {
        setError(t("common.error"));
      } finally {
        setIndexing(false);
      }
    },
    [onWorkspaceChange, t],
  );

  const selectVault = async () => {
    setError(null);

    try {
      const path = await pickVaultFolder();
      if (!path) {
        return;
      }
      await activateAndIndex(path);
    } catch {
      setError(t("common.error"));
    }
  };

  const switchTo = async (path: string) => {
    if (path === workspace?.active) {
      setMenuOpen(false);
      return;
    }
    try {
      await setActiveWorkspace(path);
      await activateAndIndex(path);
    } catch {
      setError(t("common.error"));
      setMenuOpen(false);
    }
  };

  const forget = async (path: string) => {
    try {
      await forgetWorkspace(path);
      setMenuOpen(false);
      onWorkspaceChange();
    } catch {
      setError(t("common.error"));
    }
  };

  const activeName = workspace?.active ? baseName(workspace.active) : null;
  const recents = workspace?.recents ?? [];

  return (
    <aside className="vault-panel" aria-label={t("nav.workspace")}>
      <div className="panel-header compact">
        <div>
          <p className="eyebrow">{t("workspace.switcher")}</p>
          <h2 title={workspace?.active ?? undefined}>
            {activeName ?? t("workspace.none")}
          </h2>
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

      {recents.length > 1 ? (
        <div className="workspace-switcher" ref={menuRef}>
          <button
            aria-expanded={menuOpen}
            aria-haspopup="listbox"
            className="ws-toggle"
            onClick={() => setMenuOpen((open) => !open)}
            type="button"
          >
            <span className="ws-toggle-label">{t("workspace.recents")}</span>
            <span aria-hidden="true" className={`ws-chevron ${menuOpen ? "is-open" : ""}`}>
              ▾
            </span>
          </button>
          {menuOpen ? (
            <div className="ws-menu" role="listbox" aria-label={t("workspace.recents")}>
              {recents.map((path) => {
                const isActive = path === workspace?.active;
                const name = baseName(path);
                return (
                  <div className={`ws-item ${isActive ? "is-active" : ""}`} key={path}>
                    <button
                      aria-label={t("workspace.switchTo", { name })}
                      className="ws-item-main"
                      disabled={indexing}
                      onClick={() => void switchTo(path)}
                      role="option"
                      aria-selected={isActive}
                      title={path}
                      type="button"
                    >
                      <span className="ws-item-name">{name}</span>
                      {isActive ? (
                        <span className="ws-badge">{t("workspace.activeBadge")}</span>
                      ) : null}
                    </button>
                    <button
                      aria-label={`${t("workspace.forget")}: ${name}`}
                      className="ws-forget"
                      disabled={indexing}
                      onClick={() => void forget(path)}
                      title={t("workspace.forgetTitle")}
                      type="button"
                    >
                      ✕
                    </button>
                  </div>
                );
              })}
            </div>
          ) : null}
        </div>
      ) : null}

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
            <h3>{group.name === ROOT_GROUP ? t("workspace.rootFolder") : group.name}</h3>
            {group.notes.map((note) => (
              <button
                aria-label={t("workspace.openNoteAria", { title: note.title })}
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
