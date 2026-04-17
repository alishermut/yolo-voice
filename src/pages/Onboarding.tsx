import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { MicSelector } from "../components/MicSelector";
import { Select } from "../components/ui/Select";
import type { GpuInfo, OnboardingPreviewResult } from "../shared/types";
import {
  cancelModelDownload,
  cancelOnboardingPreviewRecording,
  downloadModel,
  finishOnboardingPreview,
  getConfig,
  getGpuInfo,
  getModelStatus,
  onModelDownloadProgress,
  onModelStatus,
  saveConfig,
  startOnboardingPreviewRecording,
} from "../shared/platform";

interface OnboardingProps {
  onComplete: () => void;
}

type Step = "welcome" | "engine" | "download" | "test" | "done";
type PreviewState = "idle" | "recording" | "processing" | "success" | "error";

const HOTKEY_LABELS: Record<string, string> = {
  CapsLock: "Caps Lock",
  Space: "Space",
  Return: "Enter",
  BackSpace: "Backspace",
  ControlLeft: "Left Ctrl",
  ControlRight: "Right Ctrl",
  ShiftLeft: "Left Shift",
  ShiftRight: "Right Shift",
  AltLeft: "Left Alt",
  AltRight: "Right Alt",
  MetaLeft: "Left Win",
  MetaRight: "Right Win",
};

function formatHotkey(hotkey: string): string {
  if (!hotkey) return "Caps Lock";
  return hotkey
    .split("+")
    .map((part) => HOTKEY_LABELS[part] ?? part)
    .join(" + ");
}

export function Onboarding({ onComplete }: OnboardingProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>("welcome");
  const [deviceIndex, setDeviceIndex] = useState(0);
  const [transcriptionMode, setTranscriptionMode] = useState<"offline" | "cloud">("offline");
  const [cloudProvider, setCloudProvider] = useState("groq");
  const [cloudApiKey, setCloudApiKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [initializing, setInitializing] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [downloadStatus, setDownloadStatus] = useState("");
  const [downloadSpeed, setDownloadSpeed] = useState("");
  const [downloadEta, setDownloadEta] = useState("");
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [gpuInfo, setGpuInfo] = useState<GpuInfo | null>(null);
  const [hotkeyLabel, setHotkeyLabel] = useState("Caps Lock");
  const [language, setLanguage] = useState("en");
  const [offlineEngine, setOfflineEngine] = useState("parakeet");
  const [previewState, setPreviewState] = useState<PreviewState>("idle");
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewResult, setPreviewResult] = useState<OnboardingPreviewResult | null>(null);

  useEffect(() => {
    getConfig()
      .then((config) => {
        setHotkeyLabel(formatHotkey(config.hotkey));
        setLanguage(config.language || "en");
        setOfflineEngine(config.offline_engine || "parakeet");
        setTranscriptionMode(
          config.transcription_mode === "cloud" ? "cloud" : "offline",
        );
        if (config.cloud_stt_provider) {
          setCloudProvider(config.cloud_stt_provider);
        }
        if (config.cloud_stt_api_key) {
          setCloudApiKey(config.cloud_stt_api_key);
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    const unlisten = onModelDownloadProgress((progress) => {
      if (progress.status === "downloading") {
        setDownloadProgress(progress.percent);
        const dlMB = (progress.downloaded_bytes / 1_048_576).toFixed(0);
        const totalMB = (progress.total_bytes / 1_048_576).toFixed(0);
        const fileInfo = progress.file_count
          ? ` (${progress.file_index} of ${progress.file_count})`
          : "";
        setDownloadStatus(`Downloading${fileInfo}... ${dlMB} / ${totalMB} MB`);
        if (progress.speed_bytes_per_sec && progress.speed_bytes_per_sec > 0) {
          const speed = progress.speed_bytes_per_sec;
          setDownloadSpeed(
            speed >= 1_048_576
              ? `${(speed / 1_048_576).toFixed(1)} MB/s`
              : `${(speed / 1024).toFixed(0)} KB/s`,
          );
        }
        if (progress.eta_seconds !== undefined && progress.eta_seconds > 0) {
          const s = progress.eta_seconds;
          setDownloadEta(
            s < 60
              ? t("onboarding.download.remainingSeconds", { seconds: s })
              : t("onboarding.download.remainingMinutes", {
                  minutes: Math.floor(s / 60),
                  seconds: s % 60,
                }),
          );
        } else {
          setDownloadEta("");
        }
      } else if (progress.status === "complete") {
        setDownloadProgress(100);
        setDownloadStatus(t("onboarding.download.complete"));
        setDownloadSpeed("");
        setDownloadEta("");
      } else if (progress.status === "initializing") {
        setDownloading(false);
        setInitializing(true);
        setDownloadStatus(t("onboarding.download.loadingModel"));
        setDownloadSpeed("");
        setDownloadEta("");
      } else if (progress.status === "error") {
        setDownloadError(progress.error || t("onboarding.download.failed"));
        setDownloading(false);
        setInitializing(false);
      }
    });

    const unlistenStatus = onModelStatus((status) => {
      if (status === "ready") {
        setDownloading(false);
        setInitializing(false);
        setDownloadStatus(t("onboarding.download.modelReady"));
        setTimeout(() => setStep("test"), 400);
      } else if (status === "error") {
        setDownloading(false);
        setInitializing(false);
        setDownloadError(t("onboarding.download.modelFailedInit"));
      }
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenStatus.then((fn) => fn());
    };
  }, [t]);

  useEffect(() => {
    if (step === "done") {
      getGpuInfo()
        .then(setGpuInfo)
        .catch(() => setGpuInfo(null));
    }
  }, [step]);

  useEffect(() => {
    return () => {
      cancelOnboardingPreviewRecording().catch(() => {});
    };
  }, []);

  const canStartPreview = useMemo(() => {
    if (transcriptionMode === "cloud") {
      return cloudApiKey.trim().length > 0;
    }
    return true;
  }, [cloudApiKey, transcriptionMode]);

  const resetPreview = () => {
    setPreviewState("idle");
    setPreviewError(null);
    setPreviewResult(null);
  };

  const handleEngineNext = async () => {
    if (transcriptionMode === "offline") {
      try {
        const status = await getModelStatus();
        if (status === "ready") {
          setStep("test");
          return;
        }
      } catch {
        // fall through to download
      }
      setStep("download");
      return;
    }

    resetPreview();
    setStep("test");
  };

  const handleDownload = () => {
    setDownloading(true);
    setInitializing(false);
    setDownloadError(null);
    setDownloadProgress(0);
    setDownloadSpeed("");
    setDownloadEta("");
    setDownloadStatus(t("onboarding.download.starting"));

    downloadModel().catch((e) => {
      setDownloadError(String(e));
      setDownloading(false);
      setInitializing(false);
    });
  };

  const handleCancelDownload = () => {
    cancelModelDownload().catch(() => {});
    setDownloading(false);
    setInitializing(false);
    setDownloadProgress(0);
    setDownloadStatus("");
    setDownloadSpeed("");
    setDownloadEta("");
  };

  const handleStartPreview = async () => {
    if (!canStartPreview) {
      setPreviewError(t("onboarding.test.cloudKeyRequired"));
      setPreviewState("error");
      return;
    }

    setPreviewError(null);
    setPreviewResult(null);

    try {
      await startOnboardingPreviewRecording(deviceIndex);
      setPreviewState("recording");
    } catch (e) {
      setPreviewError(String(e));
      setPreviewState("error");
    }
  };

  const handleStopPreview = async () => {
    setPreviewError(null);
    setPreviewState("processing");

    try {
      const result = await finishOnboardingPreview({
        transcription_mode: transcriptionMode,
        cloud_stt_provider: cloudProvider,
        cloud_stt_api_key: cloudApiKey,
        language,
        offline_engine: offlineEngine,
      });
      setPreviewResult(result);
      setPreviewState("success");
    } catch (e) {
      setPreviewError(String(e));
      setPreviewState("error");
    }
  };

  const handleLeaveTestStep = async (target: Step) => {
    if (previewState === "recording") {
      await cancelOnboardingPreviewRecording().catch(() => {});
    }
    resetPreview();
    setStep(target);
  };

  const handleFinish = async () => {
    setSaving(true);
    try {
      const config = await getConfig();
      const newConfig = {
        ...config,
        device_index: deviceIndex,
        transcription_mode: transcriptionMode,
        cloud_stt_provider: cloudProvider,
        cloud_stt_api_key: cloudApiKey,
        onboarding_completed: true,
      };
      await saveConfig(newConfig);
      onComplete();
    } catch (e) {
      console.error("Onboarding save error:", e);
      onComplete();
    }
  };

  return (
    <div className="min-h-screen bg-gray-950 text-gray-100 flex items-center justify-center p-6">
      <div className="max-w-2xl w-full">
        <div className="mb-6 flex items-center justify-between text-xs text-gray-500">
          <span>{t("onboarding.progress.label")}</span>
          <span>{t(`onboarding.progress.${step}`)}</span>
        </div>

        <div className="rounded-3xl border border-gray-800 bg-gray-900/80 backdrop-blur px-8 py-8 shadow-2xl">
          {step === "welcome" && (
            <div className="space-y-8">
              <div className="space-y-3">
                <div className="inline-flex items-center rounded-full border border-emerald-500/30 bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-300">
                  {t("onboarding.welcome.badge")}
                </div>
                <h1 className="text-3xl font-semibold tracking-tight text-white">
                  {t("onboarding.welcome.title")}
                </h1>
                <p className="max-w-xl text-sm leading-6 text-gray-300">
                  {t("onboarding.welcome.description")}
                </p>
              </div>

              <div className="grid gap-4 md:grid-cols-2">
                <div className="rounded-2xl border border-gray-800 bg-gray-950/60 p-4">
                  <div className="text-xs uppercase tracking-[0.2em] text-gray-500">
                    {t("onboarding.welcome.hotkeyLabel")}
                  </div>
                  <div className="mt-2 text-2xl font-semibold text-white">
                    {hotkeyLabel}
                  </div>
                  <p className="mt-2 text-sm text-gray-400">
                    {t("onboarding.welcome.hotkeyDescription")}
                  </p>
                </div>

                <div className="rounded-2xl border border-gray-800 bg-gray-950/60 p-4">
                  <div className="text-xs uppercase tracking-[0.2em] text-gray-500">
                    {t("onboarding.welcome.promiseLabel")}
                  </div>
                  <p className="mt-2 text-sm leading-6 text-gray-300">
                    {t("onboarding.welcome.promiseDescription")}
                  </p>
                </div>
              </div>

              <div className="space-y-3">
                <h2 className="text-lg font-semibold text-gray-100">
                  {t("onboarding.welcome.micHeading")}
                </h2>
                <MicSelector
                  deviceIndex={deviceIndex}
                  onDeviceChange={setDeviceIndex}
                />
              </div>

              <div className="flex justify-end">
                <button
                  onClick={() => setStep("engine")}
                  className="rounded-xl bg-emerald-500 px-5 py-3 text-sm font-medium text-white transition-colors hover:bg-emerald-400"
                >
                  {t("onboarding.welcome.next")}
                </button>
              </div>
            </div>
          )}

          {step === "engine" && (
            <div className="space-y-8">
              <div className="space-y-3">
                <h1 className="text-3xl font-semibold tracking-tight text-white">
                  {t("onboarding.engine.title")}
                </h1>
                <p className="max-w-xl text-sm leading-6 text-gray-300">
                  {t("onboarding.engine.description")}
                </p>
              </div>

              <div className="space-y-4">
                <label
                  className={`block rounded-2xl border p-5 transition-colors ${
                    transcriptionMode === "offline"
                      ? "border-emerald-500/50 bg-emerald-500/10"
                      : "border-gray-800 bg-gray-950/50 hover:border-gray-700"
                  }`}
                >
                  <div className="flex items-start gap-3">
                    <input
                      type="radio"
                      name="engine"
                      checked={transcriptionMode === "offline"}
                      onChange={() => setTranscriptionMode("offline")}
                      className="mt-1 accent-emerald-500"
                    />
                    <div className="space-y-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="text-base font-medium text-white">
                          {t("onboarding.engine.offlineLabel")}
                        </span>
                        <span className="rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-300">
                          {t("onboarding.engine.recommended")}
                        </span>
                      </div>
                      <p className="text-sm leading-6 text-gray-300">
                        {t("onboarding.engine.offlineDescription")}
                      </p>
                      <p className="text-xs leading-5 text-emerald-200/80">
                        {t("onboarding.engine.offlineTrust")}
                      </p>
                    </div>
                  </div>
                </label>

                <label
                  className={`block rounded-2xl border p-5 transition-colors ${
                    transcriptionMode === "cloud"
                      ? "border-blue-500/50 bg-blue-500/10"
                      : "border-gray-800 bg-gray-950/40 hover:border-gray-700"
                  }`}
                >
                  <div className="flex items-start gap-3">
                    <input
                      type="radio"
                      name="engine"
                      checked={transcriptionMode === "cloud"}
                      onChange={() => setTranscriptionMode("cloud")}
                      className="mt-1 accent-blue-500"
                    />
                    <div className="space-y-2">
                      <span className="text-base font-medium text-white">
                        {t("onboarding.engine.cloudLabel")}
                      </span>
                      <p className="text-sm leading-6 text-gray-300">
                        {t("onboarding.engine.cloudDescription")}
                      </p>
                      <p className="text-xs leading-5 text-blue-200/80">
                        {t("onboarding.engine.cloudTrust")}
                      </p>
                    </div>
                  </div>
                </label>
              </div>

              {transcriptionMode === "cloud" && (
                <div className="rounded-2xl border border-gray-800 bg-gray-950/60 p-4 space-y-4">
                  <div className="flex items-center gap-3">
                    <span className="w-24 text-sm text-gray-400">
                      {t("onboarding.engine.providerLabel")}
                    </span>
                    <Select
                      value={cloudProvider}
                      onChange={(v) => setCloudProvider(v)}
                      options={[
                        { value: "groq", label: t("onboarding.engine.providerGroq") },
                        { value: "deepgram", label: t("onboarding.engine.providerDeepgram") },
                      ]}
                      className="flex-1"
                    />
                  </div>

                  <div className="flex items-center gap-3">
                    <span className="w-24 text-sm text-gray-400">
                      {t("onboarding.engine.apiKeyLabel")}
                    </span>
                    <input
                      type="password"
                      value={cloudApiKey}
                      onChange={(e) => setCloudApiKey(e.target.value)}
                      placeholder={t("onboarding.engine.apiKeyPlaceholder")}
                      className="flex-1 rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm text-gray-100 focus:outline-none focus:border-blue-500"
                    />
                  </div>
                </div>
              )}

              <div className="flex gap-3">
                <button
                  onClick={() => setStep("welcome")}
                  className="rounded-xl bg-gray-800 px-4 py-3 text-sm font-medium text-gray-200 transition-colors hover:bg-gray-700"
                >
                  {t("onboarding.engine.back")}
                </button>
                <button
                  onClick={handleEngineNext}
                  disabled={saving}
                  className="flex-1 rounded-xl bg-emerald-500 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-emerald-400 disabled:opacity-50"
                >
                  {saving ? t("onboarding.engine.settingUp") : t("onboarding.engine.next")}
                </button>
              </div>
            </div>
          )}

          {step === "download" && (
            <div className="space-y-8">
              <div className="space-y-3">
                <h1 className="text-3xl font-semibold tracking-tight text-white">
                  {t("onboarding.download.title")}
                </h1>
                <p className="max-w-xl text-sm leading-6 text-gray-300">
                  {t("onboarding.download.description")}
                </p>
              </div>

              {downloadError && (
                <div className="rounded-2xl border border-red-700 bg-red-950/40 px-4 py-3 text-sm text-red-200">
                  {downloadError}
                </div>
              )}

              {!downloading && !initializing && !downloadError && (
                <div className="space-y-4">
                  <div className="rounded-2xl border border-gray-800 bg-gray-950/60 p-5 space-y-2">
                    <h2 className="text-base font-medium text-white">
                      {t("onboarding.download.modelName")}
                    </h2>
                    <p className="text-sm leading-6 text-gray-300">
                      {t("onboarding.download.modelDescription")}
                    </p>
                  </div>

                  <button
                    onClick={handleDownload}
                    className="w-full rounded-xl bg-emerald-500 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-emerald-400"
                  >
                    {t("onboarding.download.button")}
                  </button>
                </div>
              )}

              {downloading && (
                <div className="space-y-4 rounded-2xl border border-gray-800 bg-gray-950/60 p-5">
                  <div className="h-3 w-full overflow-hidden rounded-full bg-gray-800">
                    <div
                      className="h-full rounded-full bg-emerald-500 transition-all duration-300"
                      style={{ width: `${downloadProgress}%` }}
                    />
                  </div>
                  <p className="text-sm text-gray-200">{downloadStatus}</p>
                  {(downloadSpeed || downloadEta) && (
                    <p className="text-xs text-gray-500">
                      {downloadSpeed}
                      {downloadSpeed && downloadEta ? " - " : ""}
                      {downloadEta}
                    </p>
                  )}
                  <button
                    onClick={handleCancelDownload}
                    className="rounded-xl bg-gray-800 px-4 py-2 text-sm font-medium text-gray-200 transition-colors hover:bg-gray-700"
                  >
                    {t("onboarding.download.cancelButton")}
                  </button>
                </div>
              )}

              {initializing && (
                <div className="rounded-2xl border border-blue-800/50 bg-blue-950/30 p-5 space-y-2">
                  <p className="text-sm font-medium text-blue-200">
                    {t("onboarding.download.initializingMessage")}
                  </p>
                  <p className="text-xs text-blue-100/70">
                    {t("onboarding.download.initializingNote")}
                  </p>
                </div>
              )}

              <div className="flex gap-3">
                <button
                  onClick={() => setStep("engine")}
                  className="rounded-xl bg-gray-800 px-4 py-3 text-sm font-medium text-gray-200 transition-colors hover:bg-gray-700"
                >
                  {t("onboarding.download.back")}
                </button>

                {downloadError && (
                  <button
                    onClick={handleDownload}
                    className="flex-1 rounded-xl bg-emerald-500 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-emerald-400"
                  >
                    {t("onboarding.download.retryButton")}
                  </button>
                )}
              </div>
            </div>
          )}

          {step === "test" && (
            <div className="space-y-8">
              <div className="space-y-3">
                <h1 className="text-3xl font-semibold tracking-tight text-white">
                  {t("onboarding.test.title")}
                </h1>
                <p className="max-w-xl text-sm leading-6 text-gray-300">
                  {t("onboarding.test.description")}
                </p>
              </div>

              <div className="rounded-2xl border border-gray-800 bg-gray-950/60 p-5 space-y-4">
                <div>
                  <div className="text-xs uppercase tracking-[0.2em] text-gray-500">
                    {t("onboarding.test.promptLabel")}
                  </div>
                  <p className="mt-2 text-lg font-medium text-white">
                    {t("onboarding.test.prompt")}
                  </p>
                </div>
                <p className="text-sm leading-6 text-gray-400">
                  {t("onboarding.test.helper")}
                </p>
              </div>

              {previewError && (
                <div className="rounded-2xl border border-red-700 bg-red-950/40 px-4 py-3 text-sm text-red-200">
                  {previewError}
                </div>
              )}

              <div className="rounded-2xl border border-gray-800 bg-gray-950/60 p-5 space-y-4">
                <div className="flex items-center justify-between">
                  <div>
                    <div className="text-sm font-medium text-white">
                      {t(`onboarding.test.state.${previewState}`)}
                    </div>
                    <p className="text-xs text-gray-500">
                      {t("onboarding.test.stateNote")}
                    </p>
                  </div>
                  <div
                    className={`h-3 w-3 rounded-full ${
                      previewState === "recording"
                        ? "bg-red-400 animate-pulse"
                        : previewState === "processing"
                          ? "bg-blue-400"
                          : previewState === "success"
                            ? "bg-emerald-400"
                            : previewState === "error"
                              ? "bg-red-400"
                              : "bg-gray-600"
                    }`}
                  />
                </div>

                {previewResult && (
                  <div className="rounded-2xl border border-emerald-500/30 bg-emerald-500/10 p-4">
                    <div className="text-xs uppercase tracking-[0.2em] text-emerald-300">
                      {t("onboarding.test.resultLabel")}
                    </div>
                    <p className="mt-3 text-sm leading-6 text-white">
                      {previewResult.transcript}
                    </p>
                    <p className="mt-3 text-xs text-emerald-200/80">
                      {t("onboarding.test.resultProvider", {
                        provider: previewResult.effective_provider,
                      })}
                    </p>
                  </div>
                )}

                <div className="flex flex-wrap gap-3">
                  {previewState !== "recording" && (
                    <button
                      onClick={handleStartPreview}
                      disabled={previewState === "processing"}
                      className="rounded-xl bg-emerald-500 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-emerald-400 disabled:opacity-50"
                    >
                      {previewState === "success"
                        ? t("onboarding.test.recordAgain")
                        : t("onboarding.test.startButton")}
                    </button>
                  )}

                  {previewState === "recording" && (
                    <button
                      onClick={handleStopPreview}
                      className="rounded-xl bg-red-500 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-red-400"
                    >
                      {t("onboarding.test.stopButton")}
                    </button>
                  )}

                  {previewState === "processing" && (
                    <button
                      disabled
                      className="rounded-xl bg-blue-500/70 px-4 py-3 text-sm font-medium text-white opacity-90"
                    >
                      {t("onboarding.test.processingButton")}
                    </button>
                  )}
                </div>
              </div>

              <div className="flex gap-3">
                <button
                  onClick={() => handleLeaveTestStep("engine")}
                  disabled={previewState === "processing"}
                  className="rounded-xl bg-gray-800 px-4 py-3 text-sm font-medium text-gray-200 transition-colors hover:bg-gray-700 disabled:opacity-50"
                >
                  {t("onboarding.test.back")}
                </button>
                <button
                  onClick={() => setStep("done")}
                  disabled={previewState !== "success"}
                  className="flex-1 rounded-xl bg-emerald-500 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-emerald-400 disabled:opacity-50"
                >
                  {t("onboarding.test.next")}
                </button>
              </div>
            </div>
          )}

          {step === "done" && (
            <div className="space-y-8">
              <div className="space-y-3">
                <div className="inline-flex items-center rounded-full border border-emerald-500/30 bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-300">
                  {t("onboarding.done.badge")}
                </div>
                <h1 className="text-3xl font-semibold tracking-tight text-white">
                  {t("onboarding.done.title")}
                </h1>
                <p className="max-w-xl text-sm leading-6 text-gray-300">
                  {t("onboarding.done.description")}
                </p>
              </div>

              <div className="rounded-2xl border border-gray-800 bg-gray-950/60 p-5 space-y-3">
                <div className="flex justify-between gap-4 text-sm">
                  <span className="text-gray-400">{t("onboarding.done.engineLabel")}</span>
                  <span className="text-white">
                    {transcriptionMode === "offline"
                      ? t("onboarding.done.engineOffline")
                      : t("onboarding.done.engineCloud")}
                  </span>
                </div>

                {transcriptionMode === "offline" && gpuInfo && (
                  <div className="flex justify-between gap-4 text-sm">
                    <span className="text-gray-400">{t("onboarding.done.accelerationLabel")}</span>
                    <span className={gpuInfo.available ? "text-emerald-300" : "text-yellow-300"}>
                      {gpuInfo.available
                        ? t("onboarding.done.accelerationGpu", {
                            provider: gpuInfo.execution_provider,
                          })
                        : t("onboarding.done.accelerationCpu")}
                    </span>
                  </div>
                )}

                {transcriptionMode === "cloud" && (
                  <div className="flex justify-between gap-4 text-sm">
                    <span className="text-gray-400">{t("onboarding.done.providerLabel")}</span>
                    <span className="text-white">
                      {cloudProvider === "groq"
                        ? t("onboarding.done.providerGroq")
                        : t("onboarding.done.providerDeepgram")}
                    </span>
                  </div>
                )}
              </div>

              <div className="rounded-2xl border border-gray-800 bg-gray-950/60 p-5">
                <div className="text-xs uppercase tracking-[0.2em] text-gray-500">
                  {t("onboarding.done.nextLabel")}
                </div>
                <ol className="mt-4 space-y-2 text-sm leading-6 text-gray-300">
                  <li>{t("onboarding.done.nextStep1", { hotkey: hotkeyLabel })}</li>
                  <li>{t("onboarding.done.nextStep2")}</li>
                  <li>{t("onboarding.done.nextStep3")}</li>
                </ol>
              </div>

              {transcriptionMode === "offline" && gpuInfo && !gpuInfo.available && (
                <div className="rounded-2xl border border-yellow-800/40 bg-yellow-950/30 px-4 py-3 text-xs text-yellow-200">
                  {t("onboarding.done.gpuWarning")}
                </div>
              )}

              <div className="flex gap-3">
                <button
                  onClick={() => setStep("test")}
                  disabled={saving}
                  className="rounded-xl bg-gray-800 px-4 py-3 text-sm font-medium text-gray-200 transition-colors hover:bg-gray-700 disabled:opacity-50"
                >
                  {t("onboarding.done.back")}
                </button>
                <button
                  onClick={handleFinish}
                  disabled={saving}
                  className="flex-1 rounded-xl bg-emerald-500 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-emerald-400 disabled:opacity-50"
                >
                  {saving ? t("onboarding.done.starting") : t("onboarding.done.startButton")}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
