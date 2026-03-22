// Typed Tauri invoke/listen wrappers — single source of truth for all backend calls.
// Components import from here instead of calling invoke()/listen() directly.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type {
  AppConfig,
  AppInfo,
  DeviceInfo,
  GlobalDictionary,
  GpuInfo,
  IndustryPackInfo,
  PillState,
  Profile,
  SegmentTranscribed,
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

export function getGpuAvailable(): Promise<boolean> {
  return invoke<boolean>("get_gpu_available");
}

export function getGpuInfo(): Promise<GpuInfo> {
  return invoke<GpuInfo>("get_gpu_info");
}

export function getModelStatus(): Promise<string> {
  return invoke<string>("get_model_status");
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

// ---- Sounds ----

export function previewSound(soundName: string): Promise<void> {
  return invoke("preview_sound", { soundName });
}

export function getAvailableSounds(): Promise<string[]> {
  return invoke<string[]>("get_available_sounds");
}

// ---- Global Dictionary & Industry Packs ----

export function getGlobalDictionary(): Promise<GlobalDictionary> {
  return invoke<GlobalDictionary>("get_global_dictionary");
}

export function saveGlobalDictionary(
  dictionary: GlobalDictionary,
): Promise<void> {
  return invoke("save_global_dictionary_cmd", { dictionary });
}

export function getIndustryPacks(): Promise<IndustryPackInfo[]> {
  return invoke<IndustryPackInfo[]>("get_industry_packs");
}

export function applyIndustryPack(
  packId: string,
): Promise<GlobalDictionary> {
  return invoke<GlobalDictionary>("apply_industry_pack", { packId });
}

// ---- Event Listeners ----
// Typed wrappers for Tauri events emitted by the Rust backend.

export function onRecordingState(
  handler: (state: PillState) => void,
): Promise<UnlistenFn> {
  return getCurrentWebviewWindow().listen<string>("recording-state", (event) => {
    handler(event.payload as PillState);
  });
}

export function onRecordingLevel(
  handler: (level: number) => void,
): Promise<UnlistenFn> {
  return getCurrentWebviewWindow().listen<number>("recording-level", (event) => {
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
  return getCurrentWebviewWindow().listen<SegmentTranscribed>("segment-transcribed", (event) => {
    handler(event.payload);
  });
}

export function onModelDownloadProgress(
  handler: (progress: { status: string; percent: number; downloaded_mb: number; total_mb: number }) => void,
): Promise<UnlistenFn> {
  return listen("model-download-progress", (event) => {
    handler(event.payload as { status: string; percent: number; downloaded_mb: number; total_mb: number });
  });
}
