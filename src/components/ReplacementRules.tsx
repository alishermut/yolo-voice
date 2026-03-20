import { useState } from "react";
import type { ReplacementRule } from "../shared/types";

interface ReplacementRulesProps {
  rules: ReplacementRule[];
  onChange: (rules: ReplacementRule[]) => void;
}

export function ReplacementRules({ rules, onChange }: ReplacementRulesProps) {
  const [editIdx, setEditIdx] = useState<number | null>(null);
  const [editFind, setEditFind] = useState("");
  const [editReplace, setEditReplace] = useState("");

  const handleAdd = () => {
    setEditIdx(rules.length);
    setEditFind("");
    setEditReplace("");
  };

  const handleSave = () => {
    if (editIdx === null || !editFind.trim()) return;
    const newRules = [...rules];
    const rule = { find: editFind.trim(), replace: editReplace.trim() };
    if (editIdx >= rules.length) {
      newRules.push(rule);
    } else {
      newRules[editIdx] = rule;
    }
    onChange(newRules);
    setEditIdx(null);
  };

  const handleDelete = (idx: number) => {
    onChange(rules.filter((_, i) => i !== idx));
    if (editIdx === idx) setEditIdx(null);
  };

  const handleEdit = (idx: number) => {
    setEditIdx(idx);
    setEditFind(rules[idx].find);
    setEditReplace(rules[idx].replace);
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
        const merged = [...rules];
        for (const rule of newRules) {
          if (!merged.some((r) => r.find.toLowerCase() === rule.find.toLowerCase())) {
            merged.push(rule);
          }
        }
        onChange(merged);
      }
    };
    input.click();
  };

  return (
    <div className="space-y-2">
      {rules.length > 0 && (
        <div className="border border-gray-700 rounded-lg overflow-hidden">
          <div className="grid grid-cols-[1fr_1fr_80px] gap-px bg-gray-700 text-xs text-gray-400 font-medium">
            <div className="bg-gray-800 px-3 py-2">When I say</div>
            <div className="bg-gray-800 px-3 py-2">Replace with</div>
            <div className="bg-gray-800 px-3 py-2 text-center">Actions</div>
          </div>
          <div className="max-h-60 overflow-y-auto">
            {rules.map((rule, i) => (
              <div
                key={i}
                className="grid grid-cols-[1fr_1fr_80px] gap-px bg-gray-700 text-sm"
              >
                {editIdx === i ? (
                  <>
                    <div className="bg-gray-900 px-2 py-1">
                      <input
                        type="text"
                        value={editFind}
                        onChange={(e) => setEditFind(e.target.value)}
                        className="w-full bg-transparent text-gray-200 outline-none text-sm"
                        autoFocus
                      />
                    </div>
                    <div className="bg-gray-900 px-2 py-1">
                      <input
                        type="text"
                        value={editReplace}
                        onChange={(e) => setEditReplace(e.target.value)}
                        onKeyDown={(e) => e.key === "Enter" && handleSave()}
                        className="w-full bg-transparent text-gray-200 outline-none text-sm"
                      />
                    </div>
                    <div className="bg-gray-900 flex items-center justify-center gap-1">
                      <button
                        onClick={handleSave}
                        className="text-green-400 hover:text-green-300 text-xs px-1"
                      >
                        Save
                      </button>
                    </div>
                  </>
                ) : (
                  <>
                    <div className="bg-gray-800/50 px-3 py-2 text-gray-300">
                      {rule.find}
                    </div>
                    <div className="bg-gray-800/50 px-3 py-2 text-gray-200">
                      {rule.replace}
                    </div>
                    <div className="bg-gray-800/50 flex items-center justify-center gap-1">
                      <button
                        onClick={() => handleEdit(i)}
                        className="text-gray-400 hover:text-gray-200 text-xs px-1"
                      >
                        Edit
                      </button>
                      <button
                        onClick={() => handleDelete(i)}
                        className="text-red-400 hover:text-red-300 text-xs px-1"
                      >
                        Del
                      </button>
                    </div>
                  </>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {editIdx !== null && editIdx >= rules.length && (
        <div className="flex gap-2">
          <input
            type="text"
            value={editFind}
            onChange={(e) => setEditFind(e.target.value)}
            placeholder="When I say..."
            className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
            autoFocus
          />
          <input
            type="text"
            value={editReplace}
            onChange={(e) => setEditReplace(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSave()}
            placeholder="Replace with..."
            className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
          />
          <button
            onClick={handleSave}
            className="px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm"
          >
            Add
          </button>
          <button
            onClick={() => setEditIdx(null)}
            className="px-3 py-2 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg text-sm"
          >
            Cancel
          </button>
        </div>
      )}

      <div className="flex gap-2">
        {editIdx === null && (
          <button
            onClick={handleAdd}
            className="px-3 py-1.5 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg text-sm"
          >
            + Add Rule
          </button>
        )}
        <button
          onClick={handleCsvImport}
          className="px-3 py-1.5 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg text-sm"
        >
          Import CSV
        </button>
      </div>

      {rules.length > 0 && (
        <p className="text-xs text-gray-500">
          {rules.length} replacement rule{rules.length !== 1 ? "s" : ""} active
        </p>
      )}
    </div>
  );
}
