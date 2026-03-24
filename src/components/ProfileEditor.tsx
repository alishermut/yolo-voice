import { useState, useEffect } from "react";
import type { Profile } from "../shared/types";
import { getProfiles, saveProfile, deleteProfile } from "../shared/platform";

interface ProfileEditorProps {
  activeProfileId: string;
  onProfileChange: (profileId: string) => void;
}

export function ProfileEditor({
  activeProfileId,
  onProfileChange,
}: ProfileEditorProps) {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [editingProfile, setEditingProfile] = useState<Profile | null>(null);
  const [dictInput, setDictInput] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadProfiles();
  }, []);

  const loadProfiles = async () => {
    try {
      const result = await getProfiles();
      setProfiles(result);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleSave = async () => {
    if (!editingProfile) return;
    setError(null);
    try {
      await saveProfile(editingProfile);
      await loadProfiles();
      setEditingProfile(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDelete = async (id: string) => {
    setError(null);
    try {
      await deleteProfile(id);
      await loadProfiles();
      if (activeProfileId === id) {
        onProfileChange("general");
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const handleCreate = () => {
    const newId = `custom-${Date.now()}`;
    setEditingProfile({
      id: newId,
      name: "New Profile",
      builtin: false,
      system_prompt:
        "You are a transcription post-processor.\n\nRules:\n- Fix grammar and punctuation\n- Output ONLY the corrected text, nothing else",
      terminology_hints: [],
      tone: "neutral",
    });
    setDictInput("");
  };

  const handleEdit = (profile: Profile) => {
    setEditingProfile({ ...profile });
    setDictInput(profile.terminology_hints.join(", "));
  };

  const handleDictChange = (value: string) => {
    setDictInput(value);
    if (editingProfile) {
      const words = value
        .split(",")
        .map((w) => w.trim())
        .filter((w) => w.length > 0);
      setEditingProfile({ ...editingProfile, terminology_hints: words });
    }
  };

  // Editor modal
  if (editingProfile) {
    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-gray-200">
            {editingProfile.builtin ? "View Profile" : "Edit Profile"}
          </h3>
          <button
            onClick={() => setEditingProfile(null)}
            className="text-gray-400 hover:text-gray-200 text-sm"
          >
            Cancel
          </button>
        </div>

        {error && (
          <div className="px-3 py-2 bg-red-900/50 border border-red-700 rounded-lg text-red-300 text-xs">
            {error}
          </div>
        )}

        <div className="space-y-3">
          <div>
            <label className="text-xs text-gray-400 block mb-1">Name</label>
            <input
              type="text"
              value={editingProfile.name}
              onChange={(e) =>
                setEditingProfile({ ...editingProfile, name: e.target.value })
              }
              disabled={editingProfile.builtin}
              className="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500 disabled:opacity-50"
            />
          </div>

          <div>
            <label className="text-xs text-gray-400 block mb-1">
              System Prompt
            </label>
            <textarea
              value={editingProfile.system_prompt}
              onChange={(e) =>
                setEditingProfile({
                  ...editingProfile,
                  system_prompt: e.target.value,
                })
              }
              disabled={editingProfile.builtin}
              rows={6}
              className="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500 disabled:opacity-50 resize-y"
            />
          </div>

          <div>
            <label className="text-xs text-gray-400 block mb-1">
              Terminology Hints (comma-separated terms to preserve)
            </label>
            <input
              type="text"
              value={dictInput}
              onChange={(e) => handleDictChange(e.target.value)}
              disabled={editingProfile.builtin}
              placeholder="kubectl, React, PostgreSQL, ..."
              className="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500 disabled:opacity-50"
            />
            {editingProfile.terminology_hints.length > 0 && (
              <div className="flex flex-wrap gap-1 mt-2">
                {editingProfile.terminology_hints.map((word, i) => (
                  <span
                    key={i}
                    className="px-2 py-0.5 bg-gray-700 text-gray-300 rounded text-xs"
                  >
                    {word}
                  </span>
                ))}
              </div>
            )}
          </div>

          <div>
            <label className="text-xs text-gray-400 block mb-1">Tone</label>
            <select
              value={editingProfile.tone}
              onChange={(e) =>
                setEditingProfile({ ...editingProfile, tone: e.target.value })
              }
              disabled={editingProfile.builtin}
              className="bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500 disabled:opacity-50"
            >
              <option value="neutral">Neutral</option>
              <option value="formal">Formal</option>
              <option value="casual">Casual</option>
            </select>
          </div>
        </div>

        {!editingProfile.builtin && (
          <button
            onClick={handleSave}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm font-medium transition-colors"
          >
            Save Profile
          </button>
        )}
      </div>
    );
  }

  // Profile list
  return (
    <div className="space-y-3">
      {error && (
        <div className="px-3 py-2 bg-red-900/50 border border-red-700 rounded-lg text-red-300 text-xs">
          {error}
        </div>
      )}

      {profiles.map((profile) => (
        <div
          key={profile.id}
          className={`flex items-center justify-between p-3 rounded-lg border transition-colors ${
            activeProfileId === profile.id
              ? "bg-blue-600/10 border-blue-500/50"
              : "bg-gray-800/50 border-gray-700"
          }`}
        >
          <div
            className="flex-1 cursor-pointer"
            onClick={() => onProfileChange(profile.id)}
          >
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-gray-200">
                {profile.name}
              </span>
              {profile.builtin && (
                <span className="text-xs text-gray-500">Built-in</span>
              )}
              {activeProfileId === profile.id && (
                <span className="text-xs bg-blue-600/30 text-blue-300 px-2 py-0.5 rounded-full">
                  Active
                </span>
              )}
            </div>
            {profile.terminology_hints.length > 0 && (
              <p className="text-xs text-gray-500 mt-0.5">
                Terminology hints: {profile.terminology_hints.slice(0, 5).join(", ")}
                {profile.terminology_hints.length > 5 && "..."}
              </p>
            )}
          </div>

          <div className="flex items-center gap-2 ml-3">
            <button
              onClick={() => handleEdit(profile)}
              className="px-2 py-1 text-xs text-gray-400 hover:text-gray-200 transition-colors"
            >
              {profile.builtin ? "View" : "Edit"}
            </button>
            {!profile.builtin && (
              <button
                onClick={() => handleDelete(profile.id)}
                className="px-2 py-1 text-xs text-red-400 hover:text-red-300 transition-colors"
              >
                Delete
              </button>
            )}
          </div>
        </div>
      ))}

      <button
        onClick={handleCreate}
        className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg text-sm font-medium transition-colors"
      >
        + New Profile
      </button>
    </div>
  );
}
