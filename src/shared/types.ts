// Canonical TypeScript types matching the Rust backend.
// Every component imports from here — no inline interface duplication.

export interface AppConfig {
  hotkey: string;
  device_index: number;
  whisper_model: string;
  device: string;
  compute_type: string;
  language: string;
  offline_engine: string;
  parakeet_segmented_mode_enabled: boolean;
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
  sounds_enabled: boolean;
  start_sound: string;
  stop_sound: string;
  vad_silence_threshold_ms: number;
  text_cleanup_enabled: boolean;
  offline_accuracy_boost_enabled: boolean;
  numerals_enabled: boolean;
  ui_language: string;
  pill_pinned: boolean;
  show_dictionary_migration_notice: boolean;
  transcript_diagnostics_enabled: boolean;
  hallucination_filter_enabled: boolean;
  spoken_punctuation_enabled: boolean;
  continuous_recording_enabled: boolean;
  dictation_activation_mode: DictationActivationMode;
  auto_pause_media_enabled: boolean;

  // Command mode
  command_hotkey: string;
  command_provider: string;
  command_model: string;
  command_api_key: string;
  command_base_url: string;
  command_system_prompt: string;
  default_text_action_id: string;
  settings_experience_mode: SettingsExperienceMode;
}

export interface GpuInfo {
  available: boolean;
  execution_provider: string;
}

export interface DistilWhisperModelStatus {
  status: string;
  downloaded: boolean;
  ready: boolean;
  device?: string | null;
  gpu_available: boolean;
  runtime: string;
  message?: string | null;
}

export type DictationActivationMode = "manual" | "voice_activated";
export type SettingsExperienceMode = "simple" | "advanced";

export interface SettingsPresetDefinition {
  id: string;
  label: string;
  description: string;
  updates: Partial<AppConfig>;
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

export interface UserDictionary {
  version: number;
  user_vocabulary: string[];
  user_normalization_rules: ReplacementRule[];
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
  terminology_hints: string[];
  tone: string;
  /** Single letter A-Z for command key + letter style shortcut. */
  shortcut_key: string;
}

export interface TextAction {
  id: string;
  name: string;
  builtin: boolean;
  prompt: string;
}

export interface IndustryPack {
  id: string;
  name: string;
  description: string;
  vocabulary: string[];
  replacements: ReplacementRule[];
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
}

export interface TranscriptDiagnosticsStatus {
  enabled: boolean;
  sample_count: number;
  max_samples: number;
  db_path: string;
}

export interface SupportDiagnosticsExport {
  archive_path: string;
  file_name: string;
}

export interface TranscriptHistoryExport {
  file_path: string;
  file_name: string;
  exported_count: number;
}

export type StorageLocationKind =
  | "app_data"
  | "config"
  | "models"
  | "diagnostics"
  | "history"
  | "support_exports"
  | "profiles"
  | "text_actions";

export interface StorageOverview {
  app_data_dir: string;
  config_path: string;
  models_dir: string;
  parakeet_models_dir: string;
  distil_whisper_models_dir: string;
  diagnostics_dir: string;
  transcript_history_db_path: string;
  support_exports_dir: string;
  profiles_dir: string;
  text_actions_dir: string;
}

export interface TranscriptHistoryEntry {
  id: number;
  created_at: number;
  final_text: string | null;
  inserted_text: string | null;
  transcription_mode: string;
  stt_provider: string;
  pipeline_mode: string;
  insert_success: boolean;
}

export type PillState = "idle" | "listening" | "recording" | "transcribing" | "done";

export type ActiveMode = "dictation" | "command";

export interface ModelDownloadProgress {
  status: "downloading" | "complete" | "initializing" | "error";
  file?: string;
  file_index?: number;
  file_count?: number;
  percent: number;
  downloaded_bytes: number;
  total_bytes: number;
  speed_bytes_per_sec?: number;
  eta_seconds?: number;
  error?: string;
}

export interface OnboardingPreviewRequest {
  transcription_mode: string;
  cloud_stt_provider: string;
  cloud_stt_api_key: string;
  language: string;
  offline_engine: string;
}

export interface OnboardingPreviewResult {
  transcript: string;
  effective_provider: string;
}

export type UpdateStatus =
  | "idle"
  | "checking"
  | "downloading"
  | "ready"
  | "up-to-date"
  | "error";
