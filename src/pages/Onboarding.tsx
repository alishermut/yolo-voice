import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { MicSelector } from "../components/MicSelector";
import { Select } from "../components/ui/Select";
import type { GpuInfo } from "../shared/types";
import {
  getConfig,
  saveConfig,
  downloadModel,
  cancelModelDownload,
  getModelStatus,
  getGpuInfo,
  onModelDownloadProgress,
  onModelStatus,
} from "../shared/platform";

interface OnboardingProps {
  onComplete: () => void;
}

type Step = "welcome" | "engine" | "download" | "done";

export function Onboarding({ onComplete }: OnboardingProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>("welcome");
  const [deviceIndex, setDeviceIndex] = useState(0);
  const [transcriptionMode, setTranscriptionMode] = useState<
    "offline" | "cloud"
  >("offline");
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
              : t("onboarding.download.remainingMinutes", { minutes: Math.floor(s / 60), seconds: s % 60 }),
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
      }
    });

    const unlistenStatus = onModelStatus((status) => {
      if (status === "ready") {
        setDownloading(false);
        setInitializing(false);
        setDownloadStatus(t("onboarding.download.modelReady"));
        setTimeout(() => setStep("done"), 500);
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
  }, []);

  // Fetch GPU info when entering the "done" step
  useEffect(() => {
    if (step === "done") {
      getGpuInfo()
        .then(setGpuInfo)
        .catch(() => setGpuInfo(null));
    }
  }, [step]);

  const handleEngineNext = async () => {
    if (transcriptionMode === "offline") {
      try {
        const status = await getModelStatus();
        if (status === "ready") {
          setStep("done");
          return;
        }
      } catch {
        // Continue to download step
      }
      setStep("download");
    } else {
      setStep("done");
    }
  };

  const handleDownload = () => {
    setDownloading(true);
    setInitializing(false);
    setDownloadError(null);
    setDownloadProgress(0);
    setDownloadSpeed("");
    setDownloadEta("");
    setDownloadStatus(t("onboarding.download.starting"));

    // Fire-and-forget: command returns immediately, progress comes via events
    downloadModel().catch((e) => {
      setDownloadError(String(e));
      setDownloading(false);
    });
  };

  const handleCancelDownload = () => {
    cancelModelDownload().catch(console.error);
    setDownloading(false);
    setDownloadProgress(0);
    setDownloadStatus("");
    setDownloadSpeed("");
    setDownloadEta("");
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
      <div className="max-w-lg w-full">
        {step === "welcome" && (
          <div className="space-y-6">
            <div className="text-center space-y-2">
              <h1 className="text-3xl font-bold">{t("onboarding.welcome.title")}</h1>
              <p className="text-gray-400">
                {t("onboarding.welcome.description")}
              </p>
            </div>

            <div className="space-y-3">
              <h2 className="text-lg font-semibold text-gray-200">
                {t("onboarding.welcome.micHeading")}
              </h2>
              <MicSelector
                deviceIndex={deviceIndex}
                onDeviceChange={setDeviceIndex}
              />
            </div>

            <button
              onClick={() => setStep("engine")}
              className="w-full px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors"
            >
              {t("onboarding.welcome.next")}
            </button>
          </div>
        )}

        {step === "engine" && (
          <div className="space-y-6">
            <div className="text-center space-y-2">
              <h1 className="text-2xl font-bold">{t("onboarding.engine.title")}</h1>
              <p className="text-gray-400">
                {t("onboarding.engine.description")}
              </p>
            </div>

            <div className="space-y-3">
              <label
                className={`flex items-start gap-3 p-4 rounded-lg border cursor-pointer transition-colors ${
                  transcriptionMode === "offline"
                    ? "bg-blue-600/10 border-blue-500/50"
                    : "bg-gray-800/50 border-gray-700 hover:border-gray-600"
                }`}
              >
                <input
                  type="radio"
                  name="engine"
                  checked={transcriptionMode === "offline"}
                  onChange={() => setTranscriptionMode("offline")}
                  className="accent-blue-500 mt-1"
                />
                <div>
                  <span className="text-sm font-medium text-gray-200">
                    {t("onboarding.engine.offlineLabel")}
                  </span>
                  <p className="text-xs text-gray-500 mt-1">
                    {t("onboarding.engine.offlineDescription")}
                  </p>
                </div>
              </label>

              <label
                className={`flex items-start gap-3 p-4 rounded-lg border cursor-pointer transition-colors ${
                  transcriptionMode === "cloud"
                    ? "bg-blue-600/10 border-blue-500/50"
                    : "bg-gray-800/50 border-gray-700 hover:border-gray-600"
                }`}
              >
                <input
                  type="radio"
                  name="engine"
                  checked={transcriptionMode === "cloud"}
                  onChange={() => setTranscriptionMode("cloud")}
                  className="accent-blue-500 mt-1"
                />
                <div>
                  <span className="text-sm font-medium text-gray-200">
                    {t("onboarding.engine.cloudLabel")}
                  </span>
                  <p className="text-xs text-gray-500 mt-1">
                    {t("onboarding.engine.cloudDescription")}
                  </p>
                </div>
              </label>
            </div>

            {transcriptionMode === "cloud" && (
              <div className="space-y-3 pl-2">
                <div className="flex items-center gap-3">
                  <span className="text-sm text-gray-400 w-20">{t("onboarding.engine.providerLabel")}</span>
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
                  <span className="text-sm text-gray-400 w-20">{t("onboarding.engine.apiKeyLabel")}</span>
                  <input
                    type="password"
                    value={cloudApiKey}
                    onChange={(e) => setCloudApiKey(e.target.value)}
                    placeholder={t("onboarding.engine.apiKeyPlaceholder")}
                    className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
                  />
                </div>
              </div>
            )}

            <div className="flex gap-3">
              <button
                onClick={() => setStep("welcome")}
                className="px-4 py-3 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg font-medium transition-colors"
              >
                {t("onboarding.engine.back")}
              </button>
              <button
                onClick={handleEngineNext}
                disabled={saving}
                className="flex-1 px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors disabled:opacity-50"
              >
                {saving
                  ? t("onboarding.engine.settingUp")
                  : t("onboarding.engine.next")}
              </button>
            </div>
          </div>
        )}

        {step === "download" && (
          <div className="space-y-6">
            <div className="text-center space-y-2">
              <h1 className="text-2xl font-bold">{t("onboarding.download.title")}</h1>
              <p className="text-gray-400">
                {t("onboarding.download.description")}
              </p>
            </div>

            {downloadError && (
              <div className="px-3 py-2 bg-red-900/50 border border-red-700 rounded-lg text-red-300 text-sm">
                {downloadError}
              </div>
            )}

            {!downloading && !downloadError && (
              <div className="space-y-4">
                <div className="p-4 bg-gray-800/50 border border-gray-700 rounded-lg space-y-2">
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-gray-200 font-medium">
                      {t("onboarding.download.modelName")}
                    </span>
                  </div>
                  <p className="text-xs text-gray-500">
                    {t("onboarding.download.modelDescription")}
                  </p>
                </div>

                <button
                  onClick={handleDownload}
                  className="w-full px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors"
                >
                  {t("onboarding.download.button")}
                </button>
              </div>
            )}

            {downloading && (
              <div className="space-y-3">
                <div className="w-full bg-gray-800 rounded-full h-3 overflow-hidden">
                  <div
                    className="bg-blue-600 h-full rounded-full transition-all duration-300"
                    style={{ width: `${downloadProgress}%` }}
                  />
                </div>
                <p className="text-sm text-gray-400 text-center">
                  {downloadStatus}
                </p>
                {(downloadSpeed || downloadEta) && (
                  <p className="text-xs text-gray-500 text-center">
                    {downloadSpeed}{downloadSpeed && downloadEta ? " \u2014 " : ""}{downloadEta}
                  </p>
                )}
                <button
                  onClick={handleCancelDownload}
                  className="w-full px-4 py-2 bg-gray-700 hover:bg-gray-600 text-gray-300 rounded-lg text-sm font-medium transition-colors"
                >
                  {t("onboarding.download.cancelButton")}
                </button>
              </div>
            )}

            {initializing && (
              <div className="space-y-3">
                <div className="flex items-center justify-center gap-3">
                  <svg className="animate-spin h-5 w-5 text-blue-400" viewBox="0 0 24 24" fill="none">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                  </svg>
                  <p className="text-sm text-blue-300">
                    {t("onboarding.download.initializingMessage")}
                  </p>
                </div>
                <p className="text-xs text-gray-500 text-center">
                  {t("onboarding.download.initializingNote")}
                </p>
              </div>
            )}

            {downloadError && (
              <div className="flex gap-3">
                <button
                  onClick={() => setStep("engine")}
                  className="px-4 py-3 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg font-medium transition-colors"
                >
                  {t("onboarding.download.back")}
                </button>
                <button
                  onClick={handleDownload}
                  className="flex-1 px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors"
                >
                  {t("onboarding.download.retryButton")}
                </button>
              </div>
            )}

            {!downloading && !initializing && !downloadError && (
              <button
                onClick={() => setStep("engine")}
                className="w-full px-4 py-3 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg font-medium transition-colors"
              >
                {t("onboarding.download.back")}
              </button>
            )}
          </div>
        )}

        {step === "done" && (
          <div className="space-y-6">
            <div className="text-center space-y-2">
              <div className="text-4xl mb-2">&#10003;</div>
              <h1 className="text-2xl font-bold">{t("onboarding.done.title")}</h1>
              <p className="text-gray-400">
                {t("onboarding.done.description")}
              </p>
            </div>

            <div className="p-4 bg-gray-800/50 border border-gray-700 rounded-lg space-y-3">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">{t("onboarding.done.engineLabel")}</span>
                <span className="text-gray-200">
                  {transcriptionMode === "offline"
                    ? t("onboarding.done.engineOffline")
                    : t("onboarding.done.engineCloud")}
                </span>
              </div>
              {transcriptionMode === "offline" && gpuInfo && (
                <div className="flex justify-between text-sm">
                  <span className="text-gray-400">{t("onboarding.done.accelerationLabel")}</span>
                  <span
                    className={
                      gpuInfo.available ? "text-green-400" : "text-yellow-400"
                    }
                  >
                    {gpuInfo.available
                      ? t("onboarding.done.accelerationGpu", { provider: gpuInfo.execution_provider })
                      : t("onboarding.done.accelerationCpu")}
                  </span>
                </div>
              )}
              {transcriptionMode === "cloud" && (
                <div className="flex justify-between text-sm">
                  <span className="text-gray-400">{t("onboarding.done.providerLabel")}</span>
                  <span className="text-gray-200">
                    {cloudProvider === "groq" ? t("onboarding.done.providerGroq") : t("onboarding.done.providerDeepgram")}
                  </span>
                </div>
              )}
            </div>

            {transcriptionMode === "offline" &&
              gpuInfo &&
              !gpuInfo.available && (
                <div className="text-xs text-yellow-500/80 bg-yellow-900/20 border border-yellow-800/30 rounded-lg px-3 py-2">
                  {t("onboarding.done.gpuWarning")}
                </div>
              )}

            <button
              onClick={handleFinish}
              disabled={saving}
              className="w-full px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors disabled:opacity-50"
            >
              {saving ? t("onboarding.done.starting") : t("onboarding.done.startButton")}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
