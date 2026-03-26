import { useState, useCallback } from "react";
import type { IndustryPack, ReplacementRule } from "../shared/types";
import { generateVocabVariants } from "../shared/platform";
import { inputStyles, buttonVariants, focusRing } from "./ui/styles";

interface VariantEntry {
  find: string;
  replace: string;
  checked: boolean;
  conflict: boolean;
}

interface VocabularyEditorProps {
  vocab: IndustryPack;
  onSave: (updated: IndustryPack) => Promise<void>;
  onBack: () => void;
  isGeneral?: boolean;
  onReset?: () => Promise<void>;
  allRules?: ReplacementRule[];
}

export function VocabularyEditor({
  vocab,
  onSave,
  onBack,
  isGeneral,
  onReset,
  allRules = [],
}: VocabularyEditorProps) {
  const [pack, setPack] = useState<IndustryPack>(vocab);
  const [termInput, setTermInput] = useState("");
  const [variants, setVariants] = useState<VariantEntry[]>([]);
  const [generating, setGenerating] = useState(false);
  const [showVariants, setShowVariants] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Track which terms are expanded to show their rules
  const [expandedTerms, setExpandedTerms] = useState<Set<string>>(new Set());
  // Track which term we're generating "add more" variants for (null = new term via input)
  const [addMoreTerm, setAddMoreTerm] = useState<string | null>(null);

  // Collect all existing find values across this pack and other packs
  const existingFinds = useCallback(() => {
    const set = new Set<string>();
    for (const r of pack.replacements) {
      set.add(r.find.toLowerCase());
    }
    for (const r of allRules) {
      set.add(r.find.toLowerCase());
    }
    return set;
  }, [pack.replacements, allRules]);

  // Group rules by their replace target (the term they belong to)
  const rulesByTerm = useCallback(() => {
    const map = new Map<string, ReplacementRule[]>();
    for (const rule of pack.replacements) {
      const key = rule.replace;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(rule);
    }
    return map;
  }, [pack.replacements]);

  // Rules that don't match any vocabulary term (orphaned rules)
  const orphanedRules = useCallback(() => {
    const termSet = new Set(pack.vocabulary.map((t) => t.toLowerCase()));
    return pack.replacements.filter(
      (r) => !termSet.has(r.replace.toLowerCase()),
    );
  }, [pack.replacements, pack.vocabulary]);

  const toggleTerm = (term: string) => {
    setExpandedTerms((prev) => {
      const next = new Set(prev);
      if (next.has(term)) next.delete(term);
      else next.add(term);
      return next;
    });
  };

  const handleGenerateVariants = async () => {
    const term = termInput.trim();
    if (!term) return;

    setGenerating(true);
    setError(null);
    try {
      const generated = await generateVocabVariants(term);
      const finds = existingFinds();

      const entries: VariantEntry[] = generated.map((variant) => {
        const conflict = finds.has(variant.toLowerCase());
        return {
          find: variant,
          replace: term,
          checked: !conflict,
          conflict,
        };
      });

      setAddMoreTerm(null);
      setVariants(entries);
      setShowVariants(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  // Generate more variants for an existing term, filtering out already-existing rules
  const handleAddMoreVariants = async (term: string) => {
    setAddMoreTerm(term);
    setGenerating(true);
    setError(null);
    try {
      const generated = await generateVocabVariants(term);

      // Filter out variants that already exist as rules for this term
      const existingForTerm = new Set(
        pack.replacements
          .filter((r) => r.replace.toLowerCase() === term.toLowerCase())
          .map((r) => r.find.toLowerCase()),
      );
      const allFinds = existingFinds();

      const entries: VariantEntry[] = generated
        .filter((v) => !existingForTerm.has(v.toLowerCase()))
        .map((variant) => {
          const conflict = allFinds.has(variant.toLowerCase());
          return {
            find: variant,
            replace: term,
            checked: !conflict,
            conflict,
          };
        });

      setTermInput(term);
      setVariants(entries);
      setShowVariants(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const handleAddSelected = async () => {
    const selected = variants.filter((v) => v.checked);
    const newReplacements = [
      ...pack.replacements,
      ...selected.map((v) => ({ find: v.find, replace: v.replace })),
    ];

    const term = termInput.trim();
    const newVocabulary = pack.vocabulary.includes(term)
      ? pack.vocabulary
      : [...pack.vocabulary, term];

    const updated: IndustryPack = {
      ...pack,
      replacements: newReplacements,
      vocabulary: newVocabulary,
    };

    setSaving(true);
    setError(null);
    try {
      await onSave(updated);
      setPack(updated);
      setShowVariants(false);
      setVariants([]);
      setTermInput("");
      setAddMoreTerm(null);
      // Auto-expand the newly added/updated term
      setExpandedTerms((prev) => new Set(prev).add(term));
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteRule = async (ruleToDelete: ReplacementRule) => {
    const updated: IndustryPack = {
      ...pack,
      replacements: pack.replacements.filter(
        (r) => !(r.find === ruleToDelete.find && r.replace === ruleToDelete.replace),
      ),
    };
    setSaving(true);
    setError(null);
    try {
      await onSave(updated);
      setPack(updated);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  // Remove term AND all its associated substitution rules
  const handleRemoveTerm = async (term: string) => {
    const updated: IndustryPack = {
      ...pack,
      vocabulary: pack.vocabulary.filter((t) => t !== term),
      replacements: pack.replacements.filter(
        (r) => r.replace.toLowerCase() !== term.toLowerCase(),
      ),
    };
    setSaving(true);
    setError(null);
    try {
      await onSave(updated);
      setPack(updated);
      setExpandedTerms((prev) => {
        const next = new Set(prev);
        next.delete(term);
        return next;
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleCsvImport = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".csv,.txt";
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      const text = await file.text();
      const lines = text.split("\n").filter((l) => l.trim());
      const newRules: ReplacementRule[] = [];
      for (const line of lines) {
        const parts = line.split(",").map((p) => p.trim().replace(/^"|"$/g, ""));
        if (parts.length >= 2 && parts[0]) {
          newRules.push({ find: parts[0], replace: parts[1] });
        }
      }
      if (newRules.length > 0) {
        const merged = [...pack.replacements];
        for (const rule of newRules) {
          if (!merged.some((r) => r.find.toLowerCase() === rule.find.toLowerCase())) {
            merged.push(rule);
          }
        }
        const updated: IndustryPack = { ...pack, replacements: merged };
        setSaving(true);
        setError(null);
        try {
          await onSave(updated);
          setPack(updated);
        } catch (err) {
          setError(String(err));
        } finally {
          setSaving(false);
        }
      }
    };
    input.click();
  };

  const handleReset = async () => {
    if (!onReset) return;
    if (!window.confirm(`Reset "${pack.name}" to its default contents?`)) return;
    setError(null);
    try {
      await onReset();
      onBack();
    } catch (e) {
      setError(String(e));
    }
  };

  const grouped = rulesByTerm();
  const orphans = orphanedRules();

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center gap-3">
        <button
          onClick={onBack}
          className={`px-2 py-1 bg-bg-hover hover:bg-bg-active text-text-secondary rounded-lg text-sm transition-colors ${focusRing}`}
        >
          &larr; Back
        </button>
        <h3 className="text-base font-semibold text-text-primary">{pack.name}</h3>
        <span className="text-xs text-text-muted">
          {pack.vocabulary.length} terms, {pack.replacements.length} rules
        </span>
      </div>

      {error && (
        <div className="px-3 py-2 bg-error-muted border border-error rounded-lg text-error text-xs">
          {error}
        </div>
      )}

      {/* Add Term */}
      <div className="space-y-2">
        <label className="text-sm font-medium text-text-secondary">Add Term</label>
        <div className="flex gap-2">
          <input
            type="text"
            value={termInput}
            onChange={(e) => setTermInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleGenerateVariants()}
            placeholder="e.g. TypeScript, Kubernetes..."
            className={`flex-1 ${inputStyles}`}
          />
          <button
            onClick={handleGenerateVariants}
            disabled={!termInput.trim() || generating}
            className={`${buttonVariants.primary} disabled:opacity-50 disabled:cursor-not-allowed`}
          >
            {generating ? "Generating..." : "Add"}
          </button>
        </div>
        <p className="text-xs text-text-muted">
          AI generates common misspelling variants as substitution rules.
        </p>
      </div>

      {/* Variant Confirmation Popup */}
      {showVariants && variants.length > 0 && (
        <div className="p-3 bg-bg-raised border border-border-hover rounded-lg space-y-3">
          <p className="text-sm font-medium text-text-primary">
            {addMoreTerm ? "Additional" : "Generated"} variants for &ldquo;{termInput.trim()}&rdquo;
          </p>
          <div className="space-y-1 max-h-48 overflow-y-auto">
            {variants.map((v, i) => (
              <label
                key={i}
                className="flex items-center gap-2 px-2 py-1 rounded hover:bg-bg-hover/50 cursor-pointer text-sm"
              >
                <input
                  type="checkbox"
                  checked={v.checked}
                  onChange={(e) => {
                    const next = [...variants];
                    next[i] = { ...v, checked: e.target.checked };
                    setVariants(next);
                  }}
                  className="accent-accent w-4 h-4"
                />
                <span className="text-text-secondary">{v.find}</span>
                <span className="text-text-disabled shrink-0">&rarr;</span>
                <span className="text-text-primary">{v.replace}</span>
                {v.conflict && (
                  <span className="text-warning text-xs ml-1" title="This rule already exists">
                    &#9888; conflict
                  </span>
                )}
              </label>
            ))}
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleAddSelected}
              disabled={saving || variants.every((v) => !v.checked)}
              className={`${buttonVariants.primary} px-3 py-1.5 disabled:opacity-50 disabled:cursor-not-allowed`}
            >
              {saving ? "Saving..." : "Add Selected"}
            </button>
            <button
              onClick={() => {
                setShowVariants(false);
                setVariants([]);
              }}
              className={`${buttonVariants.secondary} px-3 py-1.5`}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {showVariants && variants.length === 0 && !generating && (
        <div className="p-3 bg-bg-raised border border-border-hover rounded-lg">
          <p className="text-sm text-text-secondary">
            {addMoreTerm ? "No new variants found \u2014 all generated variants already exist." : "No variants generated for this term."}
          </p>
          <button
            onClick={() => setShowVariants(false)}
            className={`mt-2 ${buttonVariants.secondary} px-3 py-1.5`}
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Terms with grouped rules */}
      <div className="space-y-1">
        <label className="text-sm font-medium text-text-secondary">
          Terms &amp; Rules
        </label>

        {pack.vocabulary.length === 0 && orphans.length === 0 && (
          <p className="text-xs text-text-muted">No terms yet. Add one above.</p>
        )}

        {pack.vocabulary.length > 0 && (
          <div className="border border-border-default rounded-lg overflow-hidden">
            {pack.vocabulary.map((term) => {
              const rules = grouped.get(term) || [];
              const isExpanded = expandedTerms.has(term);

              return (
                <div key={term} className="border-b border-border-default/50 last:border-b-0">
                  {/* Term row */}
                  <div className="flex items-center px-3 py-2 hover:bg-bg-raised/50">
                    <button
                      onClick={() => toggleTerm(term)}
                      className="flex items-center gap-2 flex-1 min-w-0 text-left"
                    >
                      <span
                        className={`text-text-muted text-xs transition-transform ${isExpanded ? "rotate-90" : ""}`}
                      >
                        &#9654;
                      </span>
                      <span className="text-sm font-medium text-text-primary truncate">
                        {term}
                      </span>
                      <span className="text-xs text-text-muted shrink-0">
                        {rules.length} {rules.length === 1 ? "rule" : "rules"}
                      </span>
                    </button>
                    <button
                      onClick={() => handleRemoveTerm(term)}
                      disabled={saving}
                      className={`text-text-muted hover:text-error text-xs px-1.5 py-0.5 shrink-0 disabled:opacity-50 transition-colors rounded ${focusRing}`}
                      title="Remove term and all its rules"
                    >
                      &#10005;
                    </button>
                  </div>

                  {/* Expanded rules */}
                  {isExpanded && rules.length > 0 && (
                    <div className="bg-bg-base/30 border-t border-border-default/30">
                      {rules.map((rule, i) => (
                        <div
                          key={i}
                          className="flex items-center justify-between px-3 py-1.5 pl-8 text-xs border-b border-border-default/50 last:border-b-0 hover:bg-bg-raised/30"
                        >
                          <div className="flex items-center gap-2 min-w-0">
                            <span className="text-text-secondary truncate">{rule.find}</span>
                            <span className="text-text-disabled shrink-0">&rarr;</span>
                            <span className="text-text-secondary truncate">{rule.replace}</span>
                          </div>
                          <button
                            onClick={() => handleDeleteRule(rule)}
                            disabled={saving}
                            className={`text-text-disabled hover:text-error px-1 shrink-0 disabled:opacity-50 transition-colors rounded ${focusRing}`}
                            title="Remove rule"
                          >
                            &#10005;
                          </button>
                        </div>
                      ))}
                    </div>
                  )}

                  {isExpanded && rules.length === 0 && (
                    <div className="bg-bg-base/30 border-t border-border-default/30 px-3 py-1.5 pl-8">
                      <span className="text-xs text-text-disabled italic">No substitution rules for this term</span>
                    </div>
                  )}

                  {/* Add more variants button */}
                  {isExpanded && (
                    <div className="bg-bg-base/30 border-t border-border-default/30 px-3 py-1.5 pl-8">
                      <button
                        onClick={() => handleAddMoreVariants(term)}
                        disabled={generating && addMoreTerm === term}
                        className={`text-xs text-accent hover:text-accent-hover disabled:opacity-50 disabled:cursor-not-allowed transition-colors rounded ${focusRing}`}
                      >
                        {generating && addMoreTerm === term ? "Generating..." : "+ Add more rules"}
                      </button>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}

        {/* Orphaned rules (rules whose replace value doesn't match any term) */}
        {orphans.length > 0 && (
          <div className="mt-3 space-y-1">
            <label className="text-xs font-medium text-text-muted">
              Other Rules ({orphans.length})
            </label>
            <div className="border border-border-default rounded-lg overflow-hidden">
              <div className="max-h-40 overflow-y-auto">
                {orphans.map((rule, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between px-3 py-1.5 text-xs border-b border-border-default/50 last:border-b-0 hover:bg-bg-raised/50"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="text-text-secondary truncate">{rule.find}</span>
                      <span className="text-text-disabled shrink-0">&rarr;</span>
                      <span className="text-text-secondary truncate">{rule.replace}</span>
                    </div>
                    <button
                      onClick={() => handleDeleteRule(rule)}
                      disabled={saving}
                      className={`text-text-disabled hover:text-error px-1 shrink-0 disabled:opacity-50 transition-colors rounded ${focusRing}`}
                      title="Remove rule"
                    >
                      &#10005;
                    </button>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Action Buttons */}
      <div className="flex gap-2 pt-2 border-t border-border-default/50">
        <button
          onClick={handleCsvImport}
          className={`${buttonVariants.secondary} px-3 py-1.5`}
        >
          Import CSV
        </button>
        {!isGeneral && onReset && (
          <button
            onClick={handleReset}
            className={buttonVariants.danger}
          >
            Reset to Default
          </button>
        )}
      </div>
    </div>
  );
}
