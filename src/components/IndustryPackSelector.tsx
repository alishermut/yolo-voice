import { useState, useEffect } from "react";
import type { IndustryPackInfo } from "../shared/types";
import { getIndustryPacks, applyIndustryPack } from "../shared/platform";

interface IndustryPackSelectorProps {
  activePack: string;
  onApply: () => void;
}

export function IndustryPackSelector({
  activePack,
  onApply,
}: IndustryPackSelectorProps) {
  const [packs, setPacks] = useState<IndustryPackInfo[]>([]);
  const [applying, setApplying] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getIndustryPacks()
      .then(setPacks)
      .catch((e) => setError(String(e)));
  }, []);

  const handleApply = async (packId: string) => {
    setApplying(packId);
    setError(null);
    try {
      await applyIndustryPack(packId);
      onApply();
    } catch (e) {
      setError(String(e));
    } finally {
      setApplying(null);
    }
  };

  if (packs.length === 0 && !error) {
    return <p className="text-sm text-gray-500">Loading packs...</p>;
  }

  return (
    <div className="space-y-2">
      {error && (
        <div className="px-3 py-2 bg-red-900/50 border border-red-700 rounded-lg text-red-300 text-xs">
          {error}
        </div>
      )}

      <div className="grid grid-cols-2 gap-2">
        {packs.map((pack) => {
          const isActive = activePack === pack.id;
          return (
            <div
              key={pack.id}
              className={`p-3 rounded-lg border cursor-pointer transition-colors ${
                isActive
                  ? "bg-blue-600/10 border-blue-500/50"
                  : "bg-gray-800/50 border-gray-700 hover:border-gray-600"
              }`}
              onClick={() => !isActive && handleApply(pack.id)}
            >
              <div className="flex items-center justify-between mb-1">
                <span className="text-sm font-medium text-gray-200">
                  {pack.name}
                </span>
                {isActive && (
                  <span className="text-xs bg-blue-600/30 text-blue-300 px-2 py-0.5 rounded-full">
                    Active Scope
                  </span>
                )}
                {applying === pack.id && (
                  <span className="text-xs text-yellow-300">Activating...</span>
                )}
              </div>
              <p className="text-xs text-gray-500 mb-1">{pack.description}</p>
              {(pack.vocabulary_count > 0 || pack.replacement_count > 0) && (
                <p className="text-xs text-gray-600">
                  {pack.vocabulary_count > 0 &&
                    `${pack.vocabulary_count} vocab terms`}
                  {pack.vocabulary_count > 0 &&
                    pack.replacement_count > 0 &&
                    " + "}
                  {pack.replacement_count > 0 &&
                    `${pack.replacement_count} replacements`}
                </p>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
