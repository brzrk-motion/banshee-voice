//! Built-in Prompt Enhancer plugin and sidecar protocol.

use anyhow::{Context, Result, bail};
use banshee_contracts::domain::{
    PluginExecutionContext, PluginExecutionOutput, PluginManifest, PluginRuntimeState,
    PluginRuntimeStatus, PluginSettingControl, PluginSettingDefinition, PluginSettingOption,
    TextTransformPlugin,
};
use banshee_models::{ModelCapability, ModelDescriptor};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const PROMPT_ENHANCER_ID: &str = "banshee.prompt-enhancer";
pub const TARGET_MODEL_SETTING: &str = "targetModel";
pub const DEFAULT_TARGET_MODEL: &str = "gpt-5.3-codex";
pub const WORKER_PROTOCOL_VERSION: u32 = 2;
pub const MODEL_DESCRIPTOR: ModelDescriptor = ModelDescriptor {
    capability: ModelCapability::Cleanup,
    name: "Qwen2.5-1.5B-Instruct-Q4_K_M",
    directory: "llama",
    file: "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf",
    url: "https://huggingface.co/bartowski/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf",
    sha256: "1adf0b11065d8ad2e8123ea110d1ec956dab4ab038eab665614adba04b6c3370",
};

const WORKER_START_TIMEOUT: Duration = Duration::from_secs(45);
const WORKER_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const WORKER_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone)]
struct WorkerPaths {
    executable: PathBuf,
    model: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkerRequest {
    pub protocol_version: u32,
    pub request_id: u64,
    pub context: PluginExecutionContext,
    pub settings: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WorkerResponse {
    Ready {
        protocol_version: u32,
    },
    Transformed {
        request_id: u64,
        text: String,
        backend: String,
    },
    Error {
        request_id: Option<u64>,
        message: String,
    },
}

struct WorkerHandle {
    child: Child,
    stdin: ChildStdin,
    responses: Receiver<Result<WorkerResponse, String>>,
    next_request_id: u64,
}

impl WorkerHandle {
    fn stop(&mut self) {
        terminate(&mut self.child);
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Clone)]
pub struct PromptEnhancer {
    worker: Arc<Mutex<Option<WorkerHandle>>>,
    worker_paths: Arc<Mutex<Option<WorkerPaths>>>,
    worker_generation: Arc<AtomicU64>,
    last_activity: Arc<Mutex<Option<std::time::Instant>>>,
    runtime: Arc<Mutex<PluginRuntimeStatus>>,
    enabled: Arc<AtomicBool>,
}

impl Default for PromptEnhancer {
    fn default() -> Self {
        Self {
            worker: Arc::new(Mutex::new(None)),
            worker_paths: Arc::new(Mutex::new(None)),
            worker_generation: Arc::new(AtomicU64::new(0)),
            last_activity: Arc::new(Mutex::new(None)),
            runtime: Arc::new(Mutex::new(PluginRuntimeStatus {
                state: PluginRuntimeState::Missing,
                downloaded_bytes: 0,
                total_bytes: None,
                message: None,
            })),
            enabled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl PromptEnhancer {
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_runtime_status(&self, status: PluginRuntimeStatus) {
        *self.runtime.lock().expect("plugin status mutex poisoned") = status;
    }

    pub fn start_worker(&self, executable: &Path, model: &Path) -> Result<()> {
        if !self.is_enabled() {
            bail!("prompt enhancer was disabled during setup");
        }
        *self
            .worker_paths
            .lock()
            .expect("plugin worker paths mutex poisoned") = Some(WorkerPaths {
            executable: executable.to_path_buf(),
            model: model.to_path_buf(),
        });
        self.stop_worker();
        let mut child = spawn_worker(executable, model)?;
        let stdin = child.stdin.take().context("worker stdin was unavailable")?;
        let stdout = child
            .stdout
            .take()
            .context("worker stdout was unavailable")?;
        let (sender, responses) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let response = line.map_err(|error| error.to_string()).and_then(|line| {
                    serde_json::from_str(&line).map_err(|error| error.to_string())
                });
                if sender.send(response).is_err() {
                    return;
                }
            }
            let _ = sender.send(Err("prompt enhancer worker exited".into()));
        });
        match responses.recv_timeout(WORKER_START_TIMEOUT) {
            Ok(Ok(WorkerResponse::Ready { protocol_version }))
                if protocol_version == WORKER_PROTOCOL_VERSION => {}
            Ok(Ok(WorkerResponse::Error { message, .. })) => {
                terminate(&mut child);
                bail!("prompt enhancer worker failed to initialize: {message}");
            }
            Ok(Ok(response)) => {
                terminate(&mut child);
                bail!("unexpected prompt enhancer worker response: {response:?}");
            }
            Ok(Err(error)) => {
                terminate(&mut child);
                bail!("invalid prompt enhancer worker response: {error}");
            }
            Err(_) => {
                terminate(&mut child);
                bail!("prompt enhancer worker initialization timed out");
            }
        }
        if !self.is_enabled() {
            terminate(&mut child);
            bail!("prompt enhancer was disabled during setup");
        }
        *self
            .last_activity
            .lock()
            .expect("plugin worker activity mutex poisoned") = Some(std::time::Instant::now());
        let generation = self.worker_generation.fetch_add(1, Ordering::SeqCst) + 1;
        *self.worker.lock().expect("plugin worker mutex poisoned") = Some(WorkerHandle {
            child,
            stdin,
            responses,
            next_request_id: 1,
        });
        self.spawn_idle_monitor(generation);
        Ok(())
    }

    pub fn prime_worker(&self, executable: &Path, model: &Path) -> Result<()> {
        // Verify the model once, then release the worker so it can stay off RAM until needed.
        self.start_worker(executable, model)?;
        self.stop_worker();
        Ok(())
    }

    pub fn unload(&self) {
        self.enabled.store(false, Ordering::SeqCst);
        self.stop_worker();
        self.set_runtime_status(PluginRuntimeStatus {
            state: PluginRuntimeState::Missing,
            downloaded_bytes: 0,
            total_bytes: None,
            message: None,
        });
    }

    fn stop_worker(&self) {
        if let Some(mut worker) = self
            .worker
            .lock()
            .expect("plugin worker mutex poisoned")
            .take()
        {
            worker.stop();
            self.worker_generation.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn ensure_worker(&self) -> Result<()> {
        if self
            .worker
            .lock()
            .expect("plugin worker mutex poisoned")
            .is_some()
        {
            return Ok(());
        }

        let paths = self
            .worker_paths
            .lock()
            .expect("plugin worker paths mutex poisoned")
            .clone()
            .context("prompt enhancer worker is not configured")?;
        self.start_worker(&paths.executable, &paths.model)
    }

    fn spawn_idle_monitor(&self, generation: u64) {
        let worker = Arc::clone(&self.worker);
        let enabled = Arc::clone(&self.enabled);
        let last_activity = Arc::clone(&self.last_activity);
        let worker_generation = Arc::clone(&self.worker_generation);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(WORKER_IDLE_TIMEOUT);
                if !enabled.load(Ordering::SeqCst) {
                    return;
                }
                if worker_generation.load(Ordering::SeqCst) != generation {
                    return;
                }

                let mut guard = worker.lock().expect("plugin worker mutex poisoned");
                if guard.is_none() {
                    return;
                }
                let idle_for = last_activity
                    .lock()
                    .expect("plugin worker activity mutex poisoned")
                    .map(|instant| instant.elapsed())
                    .unwrap_or_default();
                if idle_for < WORKER_IDLE_TIMEOUT {
                    continue;
                }

                if let Some(mut worker) = guard.take() {
                    worker.stop();
                }
                return;
            }
        });
    }

    fn infer(
        &self,
        context: &PluginExecutionContext,
        settings: &BTreeMap<String, String>,
    ) -> Result<PluginExecutionOutput> {
        self.ensure_worker()?;
        let mut guard = self.worker.lock().expect("plugin worker mutex poisoned");
        let worker = guard
            .as_mut()
            .context("prompt enhancer worker is not ready")?;
        let request_id = worker.next_request_id;
        worker.next_request_id += 1;
        let request = WorkerRequest {
            protocol_version: WORKER_PROTOCOL_VERSION,
            request_id,
            context: context.clone(),
            settings: settings.clone(),
        };
        serde_json::to_writer(&mut worker.stdin, &request)?;
        worker.stdin.write_all(b"\n")?;
        worker.stdin.flush()?;
        match worker.responses.recv_timeout(WORKER_REQUEST_TIMEOUT) {
            Ok(Ok(WorkerResponse::Transformed {
                request_id: response_id,
                text,
                backend,
            })) if response_id == request_id => {
                *self
                    .last_activity
                    .lock()
                    .expect("plugin worker activity mutex poisoned") =
                    Some(std::time::Instant::now());
                Ok(PluginExecutionOutput { text, backend })
            }
            Ok(Ok(WorkerResponse::Error { message, .. })) => bail!(message),
            Ok(Ok(response)) => bail!("unexpected prompt enhancer worker response: {response:?}"),
            Ok(Err(error)) => bail!("prompt enhancer worker protocol error: {error}"),
            Err(_) => bail!("prompt enhancement timed out"),
        }
    }
}

impl TextTransformPlugin for PromptEnhancer {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: PROMPT_ENHANCER_ID.into(),
            name: "Prompt Enhancer".into(),
            description: "Turns spoken ideas into clear, structured prompts for coding agents."
                .into(),
            version: env!("CARGO_PKG_VERSION").into(),
            author: "Banshee".into(),
            stage: "After transcript cleanup".into(),
            settings: vec![PluginSettingDefinition {
                key: TARGET_MODEL_SETTING.into(),
                label: "Target coding model".into(),
                description: Some(
                    "Tailor the enhanced prompt for the model that will receive it.".into(),
                ),
                control: PluginSettingControl::Select {
                    default_value: DEFAULT_TARGET_MODEL.into(),
                    options: target_model_options(),
                },
            }],
        }
    }

    fn runtime_status(&self) -> PluginRuntimeStatus {
        self.runtime
            .lock()
            .expect("plugin status mutex poisoned")
            .clone()
    }

    fn transform(
        &self,
        context: &PluginExecutionContext,
        settings: &BTreeMap<String, String>,
    ) -> Result<PluginExecutionOutput> {
        match self.infer(context, settings) {
            Ok(output) if valid_enhancement(&context.current_text, &output.text) => Ok(output),
            Ok(_) => bail!("prompt enhancer returned unchanged, unstructured, or invalid text"),
            Err(error) => {
                self.stop_worker();
                self.set_runtime_status(PluginRuntimeStatus {
                    state: PluginRuntimeState::Error,
                    downloaded_bytes: 0,
                    total_bytes: None,
                    message: Some(error.to_string()),
                });
                Err(error)
            }
        }
    }
}

pub fn target_model_options() -> Vec<PluginSettingOption> {
    [
        ("gpt-5.3-codex", "GPT-5.3-Codex"),
        ("claude-opus-5", "Claude Opus 5"),
        ("gemini-3.7-flash", "Gemini 3.7 Flash"),
        ("grok-build-0.1", "Grok Build 0.1"),
    ]
    .into_iter()
    .map(|(value, label)| PluginSettingOption {
        value: value.into(),
        label: label.into(),
    })
    .collect()
}

pub fn target_model_label(settings: &BTreeMap<String, String>) -> &'static str {
    match settings
        .get(TARGET_MODEL_SETTING)
        .map(String::as_str)
        .unwrap_or(DEFAULT_TARGET_MODEL)
    {
        "claude-opus-5" => "Claude Opus 5",
        "gemini-3.7-flash" => "Gemini 3.7 Flash",
        "grok-build-0.1" => "Grok Build 0.1",
        _ => "GPT-5.3-Codex",
    }
}

pub fn enhancement_prompt(
    context: &PluginExecutionContext,
    settings: &BTreeMap<String, String>,
) -> String {
    format!(
        "<|im_start|>system\nRewrite spoken software requests into actionable prompts for coding agents. Remove conversational filler and repetition while preserving every stated requirement, especially negations and constraints. Correct speech-to-text mistakes only when the intended wording is certain. Do not add libraries, languages, files, numeric limits, algorithms, examples, tests, documentation, configurability, or implementation details unless the user stated them. When details are missing, keep the requirement high-level. Every bullet must be directly entailed by the spoken request; omit anything that is merely a useful suggestion. Return only the rewritten prompt with exactly these headings: ## Task, ## Requirements, and ## Acceptance criteria. Under ## Task, write one direct outcome statement. Write no more than three concise requirements and no more than two concise acceptance criteria. Never copy these instructions or describe the purpose of a section.<|im_end|>\n<|im_start|>user\nTarget coding model: {}\nSpoken request:\n{}<|im_end|>\n<|im_start|>assistant\n",
        target_model_label(settings),
        context.current_text
    )
}

pub fn sanitize_enhancement(output: &str) -> String {
    #[derive(Clone, Copy)]
    enum Section {
        Other,
        Requirements,
        AcceptanceCriteria,
    }

    let mut section = Section::Other;
    let mut list_items = 0;
    let mut skipping_item = false;
    let mut lines = Vec::new();
    for line in output.trim().trim_matches('"').lines() {
        let trimmed = line.trim_end();
        let heading = trimmed.trim().to_lowercase();
        if heading.starts_with("## requirements") {
            section = Section::Requirements;
            list_items = 0;
            skipping_item = false;
            lines.push("## Requirements".to_string());
            continue;
        }
        if heading.starts_with("## acceptance criteria") {
            section = Section::AcceptanceCriteria;
            list_items = 0;
            skipping_item = false;
            lines.push("## Acceptance criteria".to_string());
            continue;
        }
        if heading.starts_with("## task") {
            section = Section::Other;
            skipping_item = false;
            lines.push("## Task".to_string());
            continue;
        }

        if is_list_item(trimmed) {
            list_items += 1;
            let limit = match section {
                Section::Requirements => 3,
                Section::AcceptanceCriteria => 2,
                Section::Other => usize::MAX,
            };
            skipping_item = list_items > limit;
        }
        if !skipping_item {
            lines.push(trimmed.to_string());
        }
    }

    lines.join("\n").trim().to_string()
}

fn is_list_item(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("- ")
        || line.starts_with("* ")
        || line
            .split_once(". ")
            .is_some_and(|(prefix, _)| prefix.chars().all(|character| character.is_ascii_digit()))
}

fn valid_enhancement(input: &str, output: &str) -> bool {
    let normalized_input = normalize_for_comparison(input);
    let normalized_output = normalize_for_comparison(output);
    if normalized_input == normalized_output {
        return false;
    }

    let required_sections = ["## task", "## requirements", "## acceptance criteria"];
    if !required_sections
        .iter()
        .all(|section| normalized_output.contains(section))
    {
        return false;
    }
    let leaked_instructions = [
        "concrete implementation requirements from the request",
        "observable conditions that establish the task is complete",
        "include a ## constraints section",
        "never answer or implement the request",
    ];
    if leaked_instructions
        .iter()
        .any(|phrase| normalized_output.contains(phrase))
    {
        return false;
    }

    let input_len = normalized_input.chars().count();
    let output_len = normalized_output.chars().count();
    let length_delta = input_len.abs_diff(output_len);
    let material_length_delta = length_delta >= 12 && length_delta * 100 >= input_len.max(1) * 15;
    let structured = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
        >= 3
        && output.lines().any(is_list_item);

    material_length_delta || structured
}

fn normalize_for_comparison(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn spawn_worker(executable: &Path, model: &Path) -> Result<Child> {
    Command::new(executable)
        .arg("--model")
        .arg(model)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| prompt_worker_spawn_error(executable, error))
}

fn prompt_worker_spawn_error(executable: &Path, error: std::io::Error) -> anyhow::Error {
    #[cfg(windows)]
    if error.raw_os_error() == Some(4551) {
        return anyhow::anyhow!(
            "Windows Smart App Control blocked the prompt enhancer worker. Turn off Smart App Control for this development environment and reboot Windows, or run a build signed by a publicly trusted certificate"
        );
    }

    anyhow::Error::new(error).context(format!(
        "failed to start prompt enhancer worker at {}",
        executable.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use banshee_contracts::domain::{ProfileSummary, RecordingOrigin};
    use banshee_models::ModelInstaller;

    fn context() -> PluginExecutionContext {
        PluginExecutionContext {
            raw_text: "raw".into(),
            cleaned_text: "clean".into(),
            current_text: "add retry handling".into(),
            profile: ProfileSummary {
                id: "agent".into(),
                name: "Agent".into(),
                slug: "agent".into(),
                description: String::new(),
                built_in: true,
            },
            vocabulary: vec![],
            active_application: "Editor".into(),
            recording_origin: RecordingOrigin::Scratch,
        }
    }

    #[test]
    fn manifest_declares_target_model_presets() {
        let manifest = PromptEnhancer::default().manifest();
        let PluginSettingControl::Select {
            default_value,
            options,
        } = &manifest.settings[0].control;
        assert_eq!(default_value, DEFAULT_TARGET_MODEL);
        assert_eq!(options.len(), 4);
        assert_eq!(options[0].label, "GPT-5.3-Codex");
    }

    #[test]
    fn worker_protocol_carries_resolved_settings() {
        let request = WorkerRequest {
            protocol_version: WORKER_PROTOCOL_VERSION,
            request_id: 42,
            context: context(),
            settings: BTreeMap::from([(TARGET_MODEL_SETTING.into(), "claude-opus-5".into())]),
        };
        let json = serde_json::to_value(request).unwrap();
        assert_eq!(json["protocolVersion"], 2);
        assert_eq!(json["requestId"], 42);
        assert_eq!(json["settings"][TARGET_MODEL_SETTING], "claude-opus-5");
    }

    #[test]
    fn prompt_contains_cleaned_text_and_selected_target() {
        let prompt = enhancement_prompt(
            &context(),
            &BTreeMap::from([(TARGET_MODEL_SETTING.into(), "claude-opus-5".into())]),
        );
        assert!(prompt.contains("add retry handling"));
        assert!(prompt.contains("Target coding model: Claude Opus 5"));
        assert!(prompt.contains("## Acceptance criteria"));
        assert!(prompt.contains("exactly these headings"));
    }

    #[test]
    fn requires_a_material_structured_transformation() {
        assert!(!valid_enhancement(
            "Add retry handling to the client.",
            " Add  retry handling to the client. "
        ));
        assert!(!valid_enhancement(
            "Add robust retry handling to the API client.",
            "Add reliable retry handling to the API client."
        ));
        assert!(valid_enhancement(
            "Add retry handling to the client.",
            "## Task\nAdd resilient API retries.\n\n## Requirements\n- Retry transient failures.\n\n## Acceptance criteria\n- Transient requests are retried."
        ));
        assert!(!valid_enhancement(
            "Add retry handling to the client.",
            "## Task\nAdd retries.\n\n## Requirements\n- Concrete implementation requirements from the request.\n\n## Acceptance criteria\n- Requests are retried."
        ));
    }

    #[test]
    fn sanitizes_section_names_and_caps_generated_bullets() {
        let output = sanitize_enhancement(
            "## Task\nImprove retries.\n\n## Requirements\n- One\n- Two\n- Three\n- Invented four\n\n## Acceptance Criteria\n- First\n- Second\n- Invented third",
        );
        assert!(output.contains("## Acceptance criteria"));
        assert!(output.contains("- Three"));
        assert!(output.contains("- Second"));
        assert!(!output.contains("Invented four"));
        assert!(!output.contains("Invented third"));
    }

    #[test]
    fn model_descriptor_uses_plugin_directory() {
        let installer = ModelInstaller::from_descriptor(Path::new("app-data"), MODEL_DESCRIPTOR);
        assert!(
            installer
                .model_path()
                .ends_with(Path::new("models/llama/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn can_restart_worker_on_demand_after_shutdown() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let script = std::env::temp_dir().join(format!(
            "banshee-fake-prompt-worker-restart-{}",
            std::process::id()
        ));
        fs::write(
            &script,
            "#!/bin/sh\necho '{\"type\":\"ready\",\"protocolVersion\":2}'\nIFS= read -r request\necho '{\"type\":\"transformed\",\"requestId\":1,\"text\":\"enhanced\",\"backend\":\"fake\"}'\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();

        let enhancer = PromptEnhancer::default();
        enhancer.enable();
        enhancer
            .start_worker(&script, Path::new("unused.gguf"))
            .unwrap();
        enhancer.stop_worker();

        let output = enhancer
            .transform(
                &context(),
                &BTreeMap::from([(TARGET_MODEL_SETTING.into(), DEFAULT_TARGET_MODEL.into())]),
            )
            .unwrap();
        assert_eq!(output.text, "enhanced");
        assert_eq!(output.backend, "fake");

        enhancer.unload();
        fs::remove_file(script).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn explains_windows_application_control_spawn_failure() {
        let error = prompt_worker_spawn_error(
            Path::new("banshee-prompt-worker.exe"),
            std::io::Error::from_raw_os_error(4551),
        );
        assert!(error.to_string().contains("Windows Smart App Control"));
        assert!(error.to_string().contains("Turn off Smart App Control"));
        assert!(error.to_string().contains("publicly trusted certificate"));
    }

    #[cfg(unix)]
    #[test]
    fn communicates_with_and_stops_worker() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let script =
            std::env::temp_dir().join(format!("banshee-fake-prompt-worker-{}", std::process::id()));
        fs::write(
            &script,
            "#!/bin/sh\necho '{\"type\":\"ready\",\"protocolVersion\":2}'\nIFS= read -r request\necho '{\"type\":\"transformed\",\"requestId\":1,\"text\":\"enhanced\",\"backend\":\"fake\"}'\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();

        let enhancer = PromptEnhancer::default();
        enhancer.enable();
        enhancer
            .start_worker(&script, Path::new("unused.gguf"))
            .unwrap();
        enhancer.set_runtime_status(PluginRuntimeStatus {
            state: PluginRuntimeState::Ready,
            downloaded_bytes: 0,
            total_bytes: None,
            message: None,
        });
        let output = enhancer
            .transform(
                &context(),
                &BTreeMap::from([(TARGET_MODEL_SETTING.into(), DEFAULT_TARGET_MODEL.into())]),
            )
            .unwrap();
        assert_eq!(output.text, "enhanced");
        assert_eq!(output.backend, "fake");

        enhancer.unload();
        assert_eq!(enhancer.runtime_status().state, PluginRuntimeState::Missing);
        fs::remove_file(script).unwrap();
    }
}
