// Canonical TypeScript types matching the Rust backend.
// Every component imports from here — no inline interface duplication.

export interface AppConfig {
  hotkey: string;
  record_mode: "hold" | "toggle";
  device_index: number;
  whisper_model: string;
  device: string;
  compute_type: string;
  language: string;
  post_processing_enabled: boolean;
  active_profile_id: string;
  llm_provider: string;
  llm_model: string;
  llm_api_key: string;
  llm_base_url: string;
  transcription_mode: string;
  cloud_stt_provider: string;
  cloud_stt_api_key: string;
  onboarding_completed: boolean;
  launch_on_startup: boolean;
  start_minimized: boolean;
  active_industry_pack: string;
  start_sound: string;
  stop_sound: string;
  vad_silence_threshold_ms: number;
  text_cleanup_enabled: boolean;
}

export interface GpuInfo {
  available: boolean;
  execution_provider: string;
}

export interface SegmentTranscribed {
  index: number;
  text: string;
  full_text: string;
}

export interface ReplacementRule {
  find: string;
  replace: string;
}

export interface GlobalDictionary {
  vocabulary: string[];
  replacements: ReplacementRule[];
}

export interface DeviceInfo {
  name: string;
  index: number;
}

export interface ModelInfo {
  name: string;
  size_mb: number;
}

export interface Profile {
  id: string;
  name: string;
  builtin: boolean;
  system_prompt: string;
  dictionary: string[];
  tone: string;
}

export interface IndustryPackInfo {
  id: string;
  name: string;
  description: string;
  vocabulary_count: number;
  replacement_count: number;
}

export interface AppInfo {
  version: string;
  name: string;
  launch_on_startup: boolean;
  log_path: string;
}

export type PillState = "idle" | "recording" | "transcribing" | "done";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "downloading"
  | "ready"
  | "up-to-date"
  | "error";
