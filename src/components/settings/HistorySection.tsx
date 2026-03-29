import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import type { TranscriptHistoryEntry } from "../../shared/types";
import {
  getTranscriptHistory,
  clearTranscriptHistory,
  deleteTranscriptEntry,
  getTranscriptEntryWords,
  addWordsToDictionary,
} from "../../shared/platform";
import { inputStyles } from "../ui/styles";

const PAGE_SIZE = 20;

export function HistorySection() {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<TranscriptHistoryEntry[]>([]);
  const [search, setSearch] = useState("");
  const [hasMore, setHasMore] = useState(false);
  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [wordPickerId, setWordPickerId] = useState<number | null>(null);
  const [words, setWords] = useState<string[]>([]);
  const [selectedWords, setSelectedWords] = useState<Set<string>>(new Set());
  const [clearing, setClearing] = useState(false);

  const load = useCallback(
    async (offset: number, append: boolean) => {
      try {
        const result = await getTranscriptHistory(
          PAGE_SIZE + 1,
          offset,
          search || undefined,
        );
        const hasNext = result.length > PAGE_SIZE;
        const page = hasNext ? result.slice(0, PAGE_SIZE) : result;
        setEntries((prev) => (append ? [...prev, ...page] : page));
        setHasMore(hasNext);
      } catch {
        // Silently fail — diagnostics may not be available
      }
    },
    [search],
  );

  useEffect(() => {
    load(0, false);
  }, [load]);

  const handleDelete = async (id: number) => {
    await deleteTranscriptEntry(id);
    setEntries((prev) => prev.filter((e) => e.id !== id));
  };

  const handleClearAll = async () => {
    if (!window.confirm(t("history.clear.confirm"))) return;
    setClearing(true);
    try {
      await clearTranscriptHistory();
      setEntries([]);
      setHasMore(false);
      setExpandedId(null);
      setWordPickerId(null);
      setWords([]);
      setSelectedWords(new Set());
    } finally {
      setClearing(false);
    }
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const handleOpenWordPicker = async (id: number) => {
    const w = await getTranscriptEntryWords(id);
    setWords(w.sort());
    setSelectedWords(new Set());
    setWordPickerId(id);
  };

  const handleAddSelectedWords = async () => {
    if (selectedWords.size > 0) {
      await addWordsToDictionary(Array.from(selectedWords));
    }
    setWordPickerId(null);
  };

  const toggleWord = (word: string) => {
    setSelectedWords((prev) => {
      const next = new Set(prev);
      if (next.has(word)) next.delete(word);
      else next.add(word);
      return next;
    });
  };

  const formatTime = (ms: number) => {
    const d = new Date(ms);
    return d.toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-end">
        <button
          onClick={handleClearAll}
          disabled={entries.length === 0 || clearing}
          className="text-sm text-text-muted hover:text-error disabled:opacity-40 transition-colors"
        >
          {clearing ? t("history.clear.clearing") : t("history.clear.button")}
        </button>
      </div>

      {/* Search */}
      <div>
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={t("history.search.placeholder")}
          className={`w-full ${inputStyles}`}
        />
      </div>

      {/* Entries */}
      {entries.length === 0 ? (
        <p className="text-sm text-text-muted text-center py-8">
          {t("history.empty")}
        </p>
      ) : (
        <div className="space-y-2">
          {entries.map((entry) => (
            <div
              key={entry.id}
              className="p-3 bg-bg-base border border-border-default rounded-lg"
            >
              {/* Header row */}
              <div className="flex items-center justify-between gap-2 mb-1">
                <div className="flex items-center gap-2">
                  <span className="text-xs text-text-muted">
                    {formatTime(entry.created_at)}
                  </span>
                  <span
                    className={`text-xs px-1.5 py-0.5 rounded ${
                      entry.pipeline_mode === "command"
                        ? "bg-purple-500/20 text-purple-400"
                        : "bg-green-500/20 text-green-400"
                    }`}
                  >
                    {entry.pipeline_mode}
                  </span>
                  <span className="text-xs text-text-muted">
                    {entry.stt_provider}
                  </span>
                  {!entry.insert_success && (
                    <span className="text-xs text-error">{t("history.entry.failed")}</span>
                  )}
                </div>
              </div>

              {/* Text */}
              <p
                className={`text-sm text-text-primary cursor-pointer ${
                  expandedId !== entry.id ? "line-clamp-2" : ""
                }`}
                onClick={() =>
                  setExpandedId(expandedId === entry.id ? null : entry.id)
                }
              >
                {entry.final_text || entry.inserted_text || (
                  <span className="text-text-muted italic">{t("history.entry.empty")}</span>
                )}
              </p>

              {/* Actions */}
              <div className="flex gap-2 mt-2">
                {(entry.final_text || entry.inserted_text) && (
                  <button
                    onClick={() =>
                      handleCopy(entry.inserted_text || entry.final_text || "")
                    }
                    className="text-xs text-text-muted hover:text-text-primary transition-colors"
                  >
                    {t("history.entry.copy")}
                  </button>
                )}
                <button
                  onClick={() => handleOpenWordPicker(entry.id)}
                  className="text-xs text-text-muted hover:text-accent transition-colors"
                >
                  {t("history.entry.addToDict")}
                </button>
                <button
                  onClick={() => handleDelete(entry.id)}
                  className="text-xs text-text-muted hover:text-error transition-colors"
                >
                  {t("history.entry.delete")}
                </button>
              </div>

              {/* Word picker */}
              {wordPickerId === entry.id && (
                <div className="mt-3 p-3 bg-bg-raised rounded-lg border border-border-default">
                  <h4 className="text-xs font-medium text-text-primary mb-2">
                    {t("history.entry.selectWords")}
                  </h4>
                  <div className="flex flex-wrap gap-1.5 mb-3">
                    {words.map((word) => (
                      <button
                        key={word}
                        onClick={() => toggleWord(word)}
                        className={`text-xs px-2 py-1 rounded-md border transition-colors ${
                          selectedWords.has(word)
                            ? "bg-accent/20 border-accent text-accent"
                            : "bg-bg-base border-border-default text-text-secondary hover:border-accent"
                        }`}
                      >
                        {word}
                      </button>
                    ))}
                  </div>
                  <div className="flex gap-2">
                    <button
                      onClick={handleAddSelectedWords}
                      disabled={selectedWords.size === 0}
                      className="text-xs px-3 py-1.5 rounded-lg bg-accent text-white hover:opacity-90 disabled:opacity-40 transition-opacity"
                    >
                      {t("history.entry.addSelected", {
                        count: selectedWords.size,
                      })}
                    </button>
                    <button
                      onClick={() => setWordPickerId(null)}
                      className="text-xs px-3 py-1.5 rounded-lg text-text-muted hover:text-text-primary transition-colors"
                    >
                      {t("history.entry.cancel")}
                    </button>
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Load more */}
      {hasMore && (
        <div className="text-center">
          <button
            onClick={() => load(entries.length, true)}
            className="text-sm text-accent hover:underline"
          >
            {t("history.loadMore")}
          </button>
        </div>
      )}
    </div>
  );
}
