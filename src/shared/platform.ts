// Typed Tauri invoke/listen wrappers — single source of truth for all backend calls.
// Components import from here instead of calling invoke()/listen() directly.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ActiveMode,
  AppConfig,
  AppInfo,
  DeviceInfo,
  DistilWhisperModelStatus,
  GpuInfo,
  IndustryPack,
  IndustryPackInfo,
  ModelDownloadProgress,
  PillState,
  Profile,
  SegmentTranscribed,
  SupportDiagnosticsExport,
  TranscriptDiagnosticsStatus,
  TranscriptHistoryEntry,
} from "./types";

// ---- Config ----

export function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export function saveConfig(newConfig: AppConfig): Promise<void> {
  return invoke("save_config_cmd", { newConfig });
}

// ---- Audio Devices ----

export function listDevices(): Promise<DeviceInfo[]> {
  return invoke<DeviceInfo[]>("list_devices");
}

export function startTest(deviceIndex: number): Promise<void> {
  return invoke("start_test", { deviceIndex });
}

export function stopTest(): Promise<void> {
  return invoke("stop_test");
}

// ---- Recording ----

export function startRecording(deviceIndex: number): Promise<void> {
  return invoke("start_recording", { deviceIndex });
}

export function stopRecording(): Promise<string> {
  return invoke<string>("stop_recording");
}

// ---- Model / Inference ----

export function downloadModel(): Promise<void> {
  return invoke("download_model_cmd");
}

export function cancelModelDownload(): Promise<void> {
  return invoke("cancel_model_download_cmd");
}

export function reloadModel(useGpu: boolean): Promise<void> {
  return invoke("reload_model_cmd", { useGpu });
}

export function deleteModel(): Promise<void> {
  return invoke("delete_model_cmd");
}

export function getGpuAvailable(): Promise<boolean> {
  return invoke<boolean>("get_gpu_available");
}

export function getGpuInfo(): Promise<GpuInfo> {
  return invoke<GpuInfo>("get_gpu_info");
}

export function getModelStatus(): Promise<string> {
  return invoke<string>("get_model_status");
}

export function openDistilWhisperModelPage(): Promise<void> {
  return invoke("open_distil_whisper_model_page_cmd");
}

export function getDistilWhisperModelStatus(): Promise<DistilWhisperModelStatus> {
  return invoke<DistilWhisperModelStatus>("get_distil_whisper_model_status");
}

export function downloadDistilWhisperModel(): Promise<DistilWhisperModelStatus> {
  return invoke<DistilWhisperModelStatus>("download_distil_whisper_model_cmd");
}

export function prepareDistilWhisperModel(): Promise<DistilWhisperModelStatus> {
  return invoke<DistilWhisperModelStatus>("prepare_distil_whisper_model_cmd");
}

export function reloadDistilWhisperModel(
  useGpu: boolean,
): Promise<DistilWhisperModelStatus> {
  return invoke<DistilWhisperModelStatus>("reload_distil_whisper_model_cmd", { useGpu });
}

export function deleteDistilWhisperModel(): Promise<DistilWhisperModelStatus> {
  return invoke<DistilWhisperModelStatus>("delete_distil_whisper_model_cmd");
}

export function onDistilWhisperStatus(
  handler: (status: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("distil-whisper-status", (event) => {
    handler(event.payload);
  });
}

export function onDistilWhisperProgress(
  handler: (message: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("distil-whisper-progress", (event) => {
    handler(event.payload);
  });
}


// ---- Profiles ----

export function getProfiles(): Promise<Profile[]> {
  return invoke<Profile[]>("get_profiles");
}

export function saveProfile(profile: Profile): Promise<void> {
  return invoke("save_profile_cmd", { profile });
}

export function deleteProfile(id: string): Promise<void> {
  return invoke("delete_profile_cmd", { id });
}

export function resetProfileToDefault(id: string): Promise<void> {
  return invoke("reset_profile_to_default", { id });
}

export function testCommandLlmConnection(
  provider: string,
  model: string,
  apiKey: string,
  baseUrl: string,
): Promise<string> {
  return invoke<string>("test_command_llm_connection", {
    provider,
    model,
    apiKey,
    baseUrl,
  });
}

export function testLlmConnection(
  provider: string,
  model: string,
  apiKey: string,
  baseUrl: string,
): Promise<string> {
  return invoke<string>("test_llm_connection", {
    provider,
    model,
    apiKey,
    baseUrl,
  });
}

// ---- Startup & App Info ----

export function setLaunchOnStartup(enable: boolean): Promise<void> {
  return invoke("set_launch_on_startup", { enable });
}

export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}

export function quitApp(): Promise<void> {
  return invoke("quit_app");
}

export type SettingsNavigationTarget = "general" | "transcription" | "history";

// ---- Sounds ----

export function previewSound(soundName: string): Promise<void> {
  return invoke("preview_sound", { soundName });
}

export function getAvailableSounds(): Promise<string[]> {
  return invoke<string[]>("get_available_sounds");
}

// ---- Industry Packs ----


export function getIndustryPacks(): Promise<IndustryPackInfo[]> {
  return invoke<IndustryPackInfo[]>("get_industry_packs");
}

export function applyIndustryPack(
  packId: string,
): Promise<void> {
  return invoke("apply_industry_pack", { packId });
}

export function loadIndustryPack(id: string): Promise<IndustryPack> {
  return invoke<IndustryPack>("load_industry_pack_cmd", { id });
}

// ---- General Vocabulary & Editable Packs ----

export function getGeneralVocabulary(): Promise<IndustryPack> {
  return invoke<IndustryPack>("get_general_vocabulary");
}

export function saveGeneralVocabulary(pack: IndustryPack): Promise<void> {
  return invoke("save_general_vocabulary_cmd", { pack });
}

export function saveIndustryPack(pack: IndustryPack): Promise<void> {
  return invoke("save_industry_pack_cmd", { pack });
}

export function resetIndustryPack(id: string): Promise<void> {
  return invoke("reset_industry_pack_cmd", { id });
}

export function generateVocabVariants(term: string): Promise<string[]> {
  return invoke<string[]>("generate_vocab_variants", { term });
}

// ---- Transcript Diagnostics ----

export function getTranscriptDiagnosticsStatus(): Promise<TranscriptDiagnosticsStatus> {
  return invoke<TranscriptDiagnosticsStatus>("get_transcript_diagnostics_status");
}

export function clearTranscriptDiagnostics(): Promise<TranscriptDiagnosticsStatus> {
  return invoke<TranscriptDiagnosticsStatus>("clear_transcript_diagnostics");
}

export function exportSupportDiagnostics(): Promise<SupportDiagnosticsExport> {
  return invoke<SupportDiagnosticsExport>("export_support_diagnostics");
}

// ---- Transcript History ----

export function getTranscriptHistory(
  limit: number,
  offset: number,
  search?: string,
): Promise<TranscriptHistoryEntry[]> {
  return invoke<TranscriptHistoryEntry[]>("get_transcript_history", {
    limit,
    offset,
    search: search || null,
  });
}

export function clearTranscriptHistory(): Promise<void> {
  return invoke("clear_transcript_history");
}

export function deleteTranscriptEntry(id: number): Promise<void> {
  return invoke("delete_transcript_entry", { id });
}

export function getTranscriptEntryWords(id: number): Promise<string[]> {
  return invoke<string[]>("get_transcript_entry_words", { id });
}

export function addWordsToDictionary(words: string[]): Promise<void> {
  return invoke("add_words_to_dictionary", { words });
}

// ---- Event Listeners ----
// Typed wrappers for Tauri events emitted by the Rust backend.

export function onRecordingState(
  handler: (state: PillState) => void,
): Promise<UnlistenFn> {
  return listen<string>("recording-state", (event) => {
    handler(event.payload as PillState);
  });
}

export function onRecordingLevel(
  handler: (level: number) => void,
): Promise<UnlistenFn> {
  return listen<number>("recording-level", (event) => {
    handler(event.payload);
  });
}

export function onAudioLevel(
  handler: (level: number) => void,
): Promise<UnlistenFn> {
  return listen<number>("audio-level", (event) => {
    handler(event.payload);
  });
}

export function onModelStatus(
  handler: (status: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("model-status", (event) => {
    handler(event.payload);
  });
}

export function onGpuFallback(
  handler: (info: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("gpu-fallback", (event) => {
    handler(event.payload);
  });
}

export function onSegmentTranscribed(
  handler: (data: SegmentTranscribed) => void,
): Promise<UnlistenFn> {
  return listen<SegmentTranscribed>("segment-transcribed", (event) => {
    handler(event.payload);
  });
}

export function onModelDownloadProgress(
  handler: (progress: ModelDownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen("model-download-progress", (event) => {
    handler(event.payload as ModelDownloadProgress);
  });
}

export function onStyleSwitched(
  handler: (profileName: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("style-switched", (event) => {
    handler(event.payload);
  });
}

export function onOpenSettingsSection(
  handler: (section: SettingsNavigationTarget) => void,
): Promise<UnlistenFn> {
  return listen<SettingsNavigationTarget>("open-settings-section", (event) => {
    handler(event.payload);
  });
}

export function onActiveMode(
  handler: (mode: ActiveMode) => void,
): Promise<UnlistenFn> {
  return listen<string>("active-mode", (event) => {
    handler(event.payload as ActiveMode);
  });
}
