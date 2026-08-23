# Data Model: Banshee Local Voice Transcription App

## Overview

The MVP data model centers on persisted dictation history, local configuration, developer vocabulary, project indexing metadata, model inventory, and application-aware routing rules. SQLite is the system of record. Large derived artifacts such as model files, optional retained audio, and repository index caches live on disk and are referenced from SQLite.

## Entities

### Transcription

**Purpose**: Stores one completed or failed dictation pipeline attempt.

**Fields**:

- `id` UUID primary key
- `created_at` UTC timestamp
- `updated_at` UTC timestamp
- `status` enum: `recorded`, `processed`, `inserted`, `clipboard_fallback`, `error`, `deleted`
- `duration_ms` integer, non-negative
- `audio_retained` boolean
- `audio_path` nullable text
- `raw_text` text, nullable only if STT failed before producing output
- `deterministic_text` text, nullable only if cleanup did not run
- `final_text` text, nullable only if pipeline failed before finalization
- `word_count` integer, non-negative
- `character_count` integer, non-negative
- `source_application` nullable text
- `window_title` nullable text
- `session_type` nullable text (`x11`, `wayland`, `windows`, `macos`, `unknown`)
- `project_id` nullable foreign key -> `projects.id`
- `profile_id` foreign key -> `profiles.id`
- `speech_model_id` nullable foreign key -> `models.id`
- `cleanup_model_id` nullable foreign key -> `models.id`
- `stt_backend` text
- `cleanup_backend` nullable text
- `acceleration_requested` text
- `acceleration_actual` text
- `stt_latency_ms` nullable integer
- `cleanup_latency_ms` nullable integer
- `total_latency_ms` nullable integer
- `output_method` enum: `direct_insert`, `clipboard_paste`, `clipboard_copy_only`, `none`
- `output_result` enum: `success`, `fallback`, `failed`
- `error_code` nullable text
- `error_message` nullable text

**Relationships**:

- Many transcriptions may belong to one project.
- Many transcriptions reference one profile.
- Many transcriptions may reference one speech model and one cleanup model.

**Validation rules**:

- `duration_ms`, `word_count`, and latency fields must be `>= 0`.
- `final_text` must be preserved whenever any usable transcript exists.
- `audio_path` must be deleted if the transcription row is deleted.

**State transitions**:

- `recorded -> processed -> inserted`
- `recorded -> processed -> clipboard_fallback`
- `recorded -> error`
- `processed -> error` only if output/insertion fails after transcript preservation
- `* -> deleted` through user action or retention cleanup

### Project

**Purpose**: Represents a local repository or workspace associated with developer-aware cleanup and indexing.

**Fields**:

- `id` UUID primary key
- `name` text
- `root_path` text, unique
- `detected_vcs` enum: `git`, `none`, `other`
- `default_profile_id` nullable foreign key -> `profiles.id`
- `index_status` enum: `never_indexed`, `queued`, `indexing`, `ready`, `degraded`, `error`
- `last_indexed_at` nullable UTC timestamp
- `last_index_duration_ms` nullable integer
- `file_count` integer default `0`
- `symbol_count` integer default `0`
- `vocabulary_term_count` integer default `0`
- `branch_name` nullable text
- `exclude_patterns_json` text
- `notes` nullable text
- `created_at` UTC timestamp
- `updated_at` UTC timestamp

**Relationships**:

- One project has many transcriptions.
- One project has many dictionary entries.
- One project has one current project index snapshot.

**Validation rules**:

- `root_path` must exist when added.
- Exclusions must be valid glob-style patterns.
- Ignore `.gitignore` and built-in heavy directories during indexing.

### ProjectIndexSnapshot

**Purpose**: Captures the latest derived lexical index metadata for a project.

**Fields**:

- `id` UUID primary key
- `project_id` foreign key -> `projects.id`, unique per active snapshot
- `version` integer
- `generated_at` UTC timestamp
- `generator_version` text
- `storage_path` text
- `term_count` integer
- `file_term_count` integer
- `symbol_term_count` integer
- `dependency_term_count` integer
- `env_term_count` integer
- `branch_term_count` integer
- `warnings_json` text

**Relationships**:

- Belongs to one project.

**Validation rules**:

- `storage_path` must point to a local cache artifact owned by the app.
- Only one snapshot per project may be marked active.

### DictionaryEntry

**Purpose**: Stores a spoken phrase mapping for deterministic cleanup.

**Fields**:

- `id` UUID primary key
- `scope` enum: `global`, `project`
- `project_id` nullable foreign key -> `projects.id`
- `spoken_form` text
- `output_form` text
- `enabled` boolean default `true`
- `priority` integer default `0`
- `match_mode` enum: `exact_phrase`, `token_fuzzy`
- `notes` nullable text
- `created_at` UTC timestamp
- `updated_at` UTC timestamp

**Relationships**:

- Global entries have no project.
- Project entries belong to one project.

**Validation rules**:

- `spoken_form` must be non-empty.
- `output_form` must be non-empty.
- `project_id` is required when `scope = project`.

### Profile

**Purpose**: Defines transcript transformation behavior and output formatting policy.

**Fields**:

- `id` UUID primary key
- `name` text, unique
- `slug` text, unique
- `built_in` boolean
- `description` text
- `live_partial_transcript` boolean
- `remove_fillers` boolean
- `resolve_corrections` boolean
- `apply_dictionary` boolean
- `apply_repository_context` boolean
- `enable_cleanup_llm` boolean
- `preserve_commands` boolean
- `preserve_punctuation` boolean
- `prefer_concise_output` boolean
- `file_reference_style` enum: `none`, `agent_at_path`, `plain_path`
- `trailing_whitespace_policy` enum: `trim`, `preserve`, `single_newline`
- `cleanup_prompt_template` nullable text
- `created_at` UTC timestamp
- `updated_at` UTC timestamp

**Relationships**:

- One profile may be referenced by many transcriptions.
- One profile may be targeted by many application rules and projects.

**Validation rules**:

- Built-in profiles are immutable except for allowed user-overrides.
- `cleanup_prompt_template` is only used when `enable_cleanup_llm = true`.

### ApplicationRule

**Purpose**: Selects a profile based on the currently active application or window context.

**Fields**:

- `id` UUID primary key
- `name` text
- `enabled` boolean default `true`
- `platform_scope` enum: `any`, `linux`, `linux_x11`, `linux_wayland`, `macos`, `windows`
- `application_match` text
- `window_title_match` nullable text
- `match_type` enum: `exact`, `contains`, `regex`
- `profile_id` foreign key -> `profiles.id`
- `priority` integer default `0`
- `created_at` UTC timestamp
- `updated_at` UTC timestamp

**Validation rules**:

- Higher `priority` wins when multiple rules match.
- Regex rules must compile before save.

### ModelRecord

**Purpose**: Describes one installed local model available to the app.

**Fields**:

- `id` UUID primary key
- `name` text
- `engine` enum: `whisper_cpp`, `llama_cpp`
- `capability` enum: `speech_to_text`, `cleanup_llm`
- `path` text, unique
- `size_bytes` integer
- `quantization` nullable text
- `language_scope` nullable text
- `architecture` nullable text
- `recommended_usage` enum: `fast`, `balanced`, `accurate`, `custom`
- `default_selected` boolean
- `installed_at` UTC timestamp
- `last_verified_at` nullable UTC timestamp
- `metadata_json` text

**Validation rules**:

- `path` must exist and be readable when verified.
- Only one default speech model and one default cleanup model may exist at a time.

### Settings

**Purpose**: Stores singleton user preferences that are not modeled as separate rows.

**Fields**:

- `id` singleton integer primary key
- `launch_at_login` boolean
- `start_minimized` boolean
- `minimize_to_tray` boolean
- `show_hud` boolean
- `play_start_sound` boolean
- `play_completion_sound` boolean
- `microphone_device_id` nullable text
- `vad_sensitivity` real
- `push_to_talk_shortcut` text
- `toggle_recording_shortcut` text
- `cancel_shortcut` text
- `repaste_previous_shortcut` text
- `speech_model_default_id` nullable foreign key -> `models.id`
- `cleanup_model_default_id` nullable foreign key -> `models.id`
- `language` text
- `acceleration_preference` enum: `auto`, `cpu`, `gpu`
- `latency_profile` enum: `fast`, `balanced`, `accurate`, `custom`
- `history_enabled` boolean
- `transcription_retention_days` nullable integer
- `audio_retention_policy` enum: `never`, `24_hours`, `forever`
- `auto_paste_enabled` boolean
- `preserve_clipboard` boolean
- `paste_delay_ms` integer
- `developer_vocabulary_enabled` boolean
- `repository_context_enabled` boolean
- `cleanup_llm_enabled` boolean
- `last_profile_id` nullable foreign key -> `profiles.id`
- `updated_at` UTC timestamp

**Validation rules**:

- Shortcut values must parse through the registered shortcut backend.
- `paste_delay_ms` must be within a safe bounded range.

## Recommended Indexes

- `transcriptions(created_at DESC)`
- `transcriptions(project_id, created_at DESC)`
- `transcriptions(profile_id, created_at DESC)`
- `transcriptions(source_application, created_at DESC)`
- FTS index over `transcriptions.raw_text` and `transcriptions.final_text`
- `dictionary_entries(scope, project_id, enabled, spoken_form)`
- `projects(root_path)` unique
- `application_rules(enabled, priority DESC)`
- `models(capability, default_selected)`

## Derived Filesystem Layout

```text
<app-data>/
├── models/
│   ├── whisper/
│   └── llama/
├── audio/
│   └── <transcription-id>.wav
├── indexes/
│   └── <project-id>/snapshot-<version>.json.zst
├── logs/
└── banshee.db
```

## Data Retention Rules

- If history is disabled, completed transcriptions should not be persisted beyond what is required to finish the current output action.
- If audio retention is `never`, temporary audio buffers must be deleted immediately after pipeline completion or failure recovery.
- A delete-all-local-data action removes SQLite data, retained audio, cached indexes, and imported app-owned metadata while leaving manually imported model files untouched unless the user explicitly requests their removal.
