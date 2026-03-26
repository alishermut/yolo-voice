import { useState, useEffect, useCallback } from "react";
import { VocabularyEditor } from "../VocabularyEditor";
import type { AppConfig, IndustryPack, IndustryPackInfo } from "../../shared/types";
import {
  applyIndustryPack,
  getConfig,
  getGeneralVocabulary,
  getIndustryPacks,
  loadIndustryPack,
  resetIndustryPack,
  saveGeneralVocabulary,
  saveIndustryPack,
} from "../../shared/platform";
import { cardStyles, cardActiveStyles } from "../ui/styles";

interface EditingVocab {
  pack: IndustryPack;
  isGeneral: boolean;
  packId?: string;
}

interface VocabCardProps {
  name: string;
  description: string;
  termCount: number;
  ruleCount: number;
  isActive: boolean;
  onClick?: () => void;
  onEdit: () => void;
}

function VocabCard({
  name,
  description,
  termCount,
  ruleCount,
  isActive,
  onClick,
  onEdit,
}: VocabCardProps) {
  return (
    <div
      className={`${
        isActive ? cardActiveStyles : `${cardStyles} hover:border-border-hover`
      } ${onClick ? "cursor-pointer" : ""}`}
      onClick={onClick}
    >
      <div className="flex items-center justify-between mb-1">
        <span className="text-sm font-medium text-text-primary">{name}</span>
        <div className="flex items-center gap-2">
          {isActive && (
            <span className="text-xs bg-accent-muted text-accent px-2 py-0.5 rounded-full">
              Active
            </span>
          )}
          <button
            onClick={(e) => {
              e.stopPropagation();
              onEdit();
            }}
            className="text-xs text-text-secondary hover:text-text-primary px-2 py-0.5 bg-bg-hover hover:bg-bg-active rounded transition-colors"
          >
            Edit
          </button>
        </div>
      </div>
      <p className="text-xs text-text-muted mb-1">{description}</p>
      {(termCount > 0 || ruleCount > 0) && (
        <p className="text-xs text-text-muted">
          {termCount > 0 && `${termCount} terms`}
          {termCount > 0 && ruleCount > 0 && " + "}
          {ruleCount > 0 && `${ruleCount} rules`}
        </p>
      )}
    </div>
  );
}

interface VocabularySectionProps {
  config: AppConfig;
  setConfig: React.Dispatch<React.SetStateAction<AppConfig | null>>;
  setError: (error: string | null) => void;
}

export function VocabularySection({
  config,
  setConfig,
  setError,
}: VocabularySectionProps) {
  const [generalVocab, setGeneralVocab] = useState<IndustryPack | null>(null);
  const [industryPacks, setIndustryPacks] = useState<IndustryPackInfo[]>([]);
  const [editingVocab, setEditingVocab] = useState<EditingVocab | null>(null);

  const loadVocabularyData = useCallback(() => {
    getGeneralVocabulary()
      .then(setGeneralVocab)
      .catch(() => {});
    getIndustryPacks()
      .then(setIndustryPacks)
      .catch(() => {});
  }, []);

  useEffect(() => {
    loadVocabularyData();
  }, [loadVocabularyData]);

  if (editingVocab) {
    return (
      <VocabularyEditor
        vocab={editingVocab.pack}
        onSave={async (updated) => {
          if (editingVocab.isGeneral) {
            await saveGeneralVocabulary(updated);
            setGeneralVocab(updated);
          } else {
            await saveIndustryPack(updated);
          }
          setEditingVocab((prev) =>
            prev ? { ...prev, pack: updated } : prev,
          );
        }}
        onBack={() => {
          setEditingVocab(null);
          loadVocabularyData();
        }}
        isGeneral={editingVocab.isGeneral}
        onReset={
          editingVocab.isGeneral
            ? undefined
            : async () => {
                if (editingVocab.packId) {
                  await resetIndustryPack(editingVocab.packId);
                  loadVocabularyData();
                }
              }
        }
        allRules={
          editingVocab.isGeneral ? [] : generalVocab?.replacements ?? []
        }
      />
    );
  }

  return (
    <div className="space-y-3">
      {/* User Vocabulary card */}
      {generalVocab && (
        <VocabCard
          name="User Vocabulary"
          description="Always active. Your personal terms and rules apply to all transcriptions."
          termCount={generalVocab.vocabulary.length}
          ruleCount={generalVocab.replacements.length}
          isActive={true}
          onEdit={() =>
            setEditingVocab({ pack: generalVocab, isGeneral: true })
          }
        />
      )}

      {/* Specialized vocabulary cards */}
      {industryPacks.map((pack) => {
        const isActive = config.active_industry_pack === pack.id;
        return (
          <VocabCard
            key={pack.id}
            name={pack.name}
            description={pack.description}
            termCount={pack.vocabulary_count}
            ruleCount={pack.replacement_count}
            isActive={isActive}
            onClick={
              isActive
                ? undefined
                : async () => {
                    try {
                      await applyIndustryPack(pack.id);
                      const cfg = await getConfig();
                      setConfig(cfg);
                      loadVocabularyData();
                    } catch (e) {
                      setError(String(e));
                    }
                  }
            }
            onEdit={async () => {
              try {
                const full = await loadIndustryPack(pack.id);
                setEditingVocab({
                  pack: full,
                  isGeneral: false,
                  packId: pack.id,
                });
              } catch (e) {
                setError(String(e));
              }
            }}
          />
        );
      })}

      {industryPacks.length === 0 && !generalVocab && (
        <p className="text-sm text-text-muted">Loading vocabulary data...</p>
      )}
    </div>
  );
}
