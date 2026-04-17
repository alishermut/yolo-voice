import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { SettingsExperienceMode, TextAction } from "../shared/types";
import {
  deleteTextAction,
  getTextActions,
  resetTextActionToDefault,
  saveTextAction,
} from "../shared/platform";
import {
  buttonVariants,
  cardStyles,
  descStyles,
  focusRing,
  inputStyles,
  textareaStyles,
} from "./ui/styles";

interface TextActionManagerProps {
  defaultActionId: string;
  settingsMode: SettingsExperienceMode;
  onDefaultActionChange: (id: string) => Promise<void>;
}

const CLEAN_UP_ID = "clean_up";

export function TextActionManager({
  defaultActionId,
  settingsMode,
  onDefaultActionChange,
}: TextActionManagerProps) {
  const { t } = useTranslation();
  const [actions, setActions] = useState<TextAction[]>([]);
  const [editingAction, setEditingAction] = useState<TextAction | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const isAdvanced = settingsMode === "advanced";

  const defaultAction = useMemo(
    () => actions.find((action) => action.id === defaultActionId) ?? null,
    [actions, defaultActionId],
  );

  async function loadActions() {
    try {
      const result = await getTextActions();
      setActions(result);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    loadActions();
  }, []);

  useEffect(() => {
    if (settingsMode !== "advanced" && editingAction) {
      setEditingAction(null);
    }
  }, [editingAction, settingsMode]);

  async function setDefaultAction(id: string) {
    try {
      await onDefaultActionChange(id);
      setPickerOpen(false);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleSaveAction() {
    if (!editingAction) return;
    setSaving(true);
    try {
      await saveTextAction(editingAction);
      await loadActions();
      setEditingAction(null);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function handleDeleteAction(action: TextAction) {
    try {
      await deleteTextAction(action.id);
      if (action.id === defaultActionId) {
        await onDefaultActionChange(CLEAN_UP_ID);
      }
      await loadActions();
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleResetAction(action: TextAction) {
    try {
      await resetTextActionToDefault(action.id);
      await loadActions();
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  if (editingAction) {
    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-text-primary">
            {editingAction.builtin
              ? t("textActions.actions.editBuiltIn")
              : t("textActions.actions.editCustom")}
          </h3>
          <button
            onClick={() => setEditingAction(null)}
            className={`text-text-secondary hover:text-text-primary text-sm rounded ${focusRing}`}
          >
            &larr; {t("textActions.actions.back")}
          </button>
        </div>

        {error && (
          <div className="px-3 py-2 bg-error-muted border border-error rounded-lg text-error text-xs">
            {error}
          </div>
        )}

        <div>
          <label className="text-xs text-text-secondary block mb-1">
            {t("textActions.actions.nameLabel")}
          </label>
          <input
            type="text"
            value={editingAction.name}
            onChange={(e) =>
              setEditingAction({ ...editingAction, name: e.target.value })
            }
            className={inputStyles}
          />
        </div>

        <div>
          <label className="text-xs text-text-secondary block mb-1">
            {t("textActions.actions.promptLabel")}
          </label>
          <textarea
            value={editingAction.prompt}
            onChange={(e) =>
              setEditingAction({ ...editingAction, prompt: e.target.value })
            }
            rows={7}
            className={textareaStyles}
          />
          <p className={`${descStyles} mt-2`}>
            {t("textActions.actions.promptHint")}
          </p>
        </div>

        <div className="flex items-center gap-3">
          <button
            onClick={handleSaveAction}
            disabled={saving}
            className={buttonVariants.primary}
          >
            {saving
              ? t("textActions.actions.saving")
              : t("textActions.actions.save")}
          </button>
          {editingAction.builtin && (
            <button
              onClick={() => handleResetAction(editingAction)}
              className={buttonVariants.secondary}
            >
              {t("textActions.actions.reset")}
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {error && (
        <div className="px-3 py-2 bg-error-muted border border-error rounded-lg text-error text-xs">
          {error}
        </div>
      )}

      <div className="p-4 rounded-lg border bg-bg-raised border-border-default space-y-3">
        <div className="flex items-start justify-between gap-3">
          <div>
            <p className="text-xs uppercase tracking-wider text-text-muted">
              {t("textActions.quickPicker.heading")}
            </p>
            <p className="text-sm font-medium text-text-primary mt-1">
              {defaultAction?.name ?? t("textActions.quickPicker.none")}
            </p>
            <p className="text-xs text-text-muted mt-1">
              {t("textActions.quickPicker.description")}
            </p>
          </div>
          <button
            onClick={() => setPickerOpen((open) => !open)}
            className={buttonVariants.secondary}
          >
            {t("textActions.quickPicker.button")}
          </button>
        </div>

        {pickerOpen && (
          <div className="grid gap-2 sm:grid-cols-2">
            {actions.map((action) => (
              <button
                key={action.id}
                onClick={() => setDefaultAction(action.id)}
                className={`text-left rounded-lg border px-3 py-2 transition-colors ${
                  action.id === defaultActionId
                    ? "border-accent bg-accent-muted"
                    : "border-border-default bg-bg-base hover:border-border-hover"
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-text-primary">
                    {action.name}
                  </span>
                  {action.builtin && (
                    <span className="text-[10px] uppercase tracking-wide text-text-muted">
                      {t("textActions.actions.builtIn")}
                    </span>
                  )}
                </div>
                <p className="text-xs text-text-muted mt-1 line-clamp-2">
                  {action.prompt}
                </p>
              </button>
            ))}
          </div>
        )}
      </div>

      {isAdvanced && (
        <div className="space-y-2">
          {actions.map((action) => {
            const isDefault = action.id === defaultActionId;
            return (
              <div key={action.id} className={`${cardStyles} space-y-3`}>
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-sm font-medium text-text-primary">
                        {action.name}
                      </span>
                      {action.builtin && (
                        <span className="text-[10px] uppercase tracking-wide text-text-muted">
                          {t("textActions.actions.builtIn")}
                        </span>
                      )}
                      {isDefault && (
                        <span className="text-[10px] uppercase tracking-wide text-accent">
                          {t("textActions.actions.default")}
                        </span>
                      )}
                    </div>
                    <p className="text-xs text-text-muted mt-2 line-clamp-3">
                      {action.prompt}
                    </p>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    {!isDefault && (
                      <button
                        onClick={() => setDefaultAction(action.id)}
                        className={buttonVariants.secondary}
                      >
                        {t("textActions.actions.setDefault")}
                      </button>
                    )}
                    <button
                      onClick={() => setEditingAction({ ...action })}
                      className={buttonVariants.secondary}
                    >
                      {t("textActions.actions.edit")}
                    </button>
                    {action.builtin ? (
                      <button
                        onClick={() => handleResetAction(action)}
                        className={buttonVariants.secondary}
                      >
                        {t("textActions.actions.reset")}
                      </button>
                    ) : (
                      <button
                        onClick={() => handleDeleteAction(action)}
                        className={buttonVariants.danger}
                      >
                        {t("textActions.actions.delete")}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {isAdvanced && (
        <button
          onClick={() =>
            setEditingAction({
              id: `custom-${Date.now()}`,
              name: t("textActions.actions.newActionName"),
              builtin: false,
              prompt: t("textActions.actions.newActionPrompt"),
            })
          }
          className={buttonVariants.secondary}
        >
          {t("textActions.actions.new")}
        </button>
      )}
    </div>
  );
}
