import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { markdown } from "@codemirror/lang-markdown";
import CodeMirror, { EditorView } from "@uiw/react-codemirror";

import { readNote, writeNote } from "../../lib/ipc";
import type { NoteRef } from "../../lib/types";

type NoteEditorProps = {
  note: NoteRef | null;
};

function basename(path: string) {
  const parts = path.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] ?? path;
}

export function NoteEditor({ note }: NoteEditorProps) {
  const [content, setContent] = useState("");
  const [savedContent, setSavedContent] = useState("");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [savedAt, setSavedAt] = useState<string | null>(null);
  const saveTimerRef = useRef<number | null>(null);
  const savingRef = useRef(false);

  const isDirty = content !== savedContent;
  const editorExtensions = useMemo(() => [markdown(), EditorView.lineWrapping], []);
  const title = useMemo(() => {
    if (!note) {
      return "Not editörü";
    }

    return note.title || basename(note.path);
  }, [note]);

  useEffect(() => {
    let alive = true;

    if (!note) {
      setContent("");
      setSavedContent("");
      setError(null);
      setSavedAt(null);
      return () => {
        alive = false;
      };
    }

    setLoading(true);
    setError(null);
    setSavedAt(null);

    void readNote(note.path)
      .then((text) => {
        if (!alive) {
          return;
        }

        setContent(text);
        setSavedContent(text);
      })
      .catch(() => {
        if (alive) {
          setError("Not açılamadı.");
          setContent("");
          setSavedContent("");
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
  }, [note]);

  const save = useCallback(async () => {
    if (!note || savingRef.current || content === savedContent) {
      return;
    }

    const path = note.path;
    const contentToSave = content;
    savingRef.current = true;
    setSaving(true);
    setError(null);

    try {
      await writeNote(path, contentToSave);
      setSavedContent(contentToSave);
      setSavedAt(new Intl.DateTimeFormat("tr-TR", {
        hour: "2-digit",
        minute: "2-digit",
      }).format(new Date()));
    } catch {
      setError("Not kaydedilemedi.");
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  }, [content, note, savedContent]);

  useEffect(() => {
    if (saveTimerRef.current !== null) {
      window.clearTimeout(saveTimerRef.current);
      saveTimerRef.current = null;
    }

    if (!note || loading || !isDirty) {
      return undefined;
    }

    saveTimerRef.current = window.setTimeout(() => {
      saveTimerRef.current = null;
      void save();
    }, 800);

    return () => {
      if (saveTimerRef.current !== null) {
        window.clearTimeout(saveTimerRef.current);
        saveTimerRef.current = null;
      }
    };
  }, [isDirty, loading, note, save]);

  if (!note) {
    return (
      <section className="editor-pane empty-editor" aria-label="Not editörü">
        <div className="empty-state large">Bir not seçin.</div>
      </section>
    );
  }

  return (
    <section className="editor-pane" aria-label="Not editörü">
      <header className="editor-header">
        <div className="editor-title">
          <p className="eyebrow">Editör</p>
          <h1>{title}</h1>
          <p className="path-label mono">{note.path}</p>
        </div>
        <div className="toolbar" aria-label="Not işlemleri">
          <span className={`save-state ${isDirty ? "is-dirty" : ""}`}>
            {loading
              ? "Yükleniyor"
              : isDirty
                ? "Kaydedilmedi"
                : savedAt
                  ? `Kaydedildi ${savedAt}`
                  : "Kaydedildi"}
          </span>
          <button
            aria-label="Notu kaydet"
            className="button primary"
            disabled={loading || saving || !isDirty}
            onClick={save}
            type="button"
          >
            {saving ? "Kaydediliyor" : "Kaydet"}
          </button>
        </div>
      </header>

      {error ? <p className="notice error">{error}</p> : null}

      <CodeMirror
        aria-label={`${title} içeriği`}
        basicSetup={{
          foldGutter: false,
          highlightActiveLineGutter: false,
          lineNumbers: false,
        }}
        className="note-codemirror"
        editable={!loading}
        extensions={editorExtensions}
        height="100%"
        onChange={setContent}
        readOnly={loading}
        theme="dark"
        value={content}
      />
    </section>
  );
}
