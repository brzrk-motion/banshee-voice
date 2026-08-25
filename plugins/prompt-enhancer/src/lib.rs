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
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const PROMPT_ENHANCER_ID: &str = "banshee.prompt-enhancer";
pub const TARGET_MODEL_SETTING: &str = "targetModel";
pub const DEFAULT_TARGET_MODEL: &str = "gpt-5.3-codex";
pub const WORKER_PROTOCOL_VERSION: u32 = 2;
pub const MODEL_DESCRIPTOR: ModelDescriptor = ModelDescriptor {
    capability: ModelCapability::Cleanup,
    name: "Qwen2.5-0.5B-Instruct-Q4_K_M",
    directory: "llama",
    file: "Qwen2.5-0.5B-Instruct-Q4_K_M.gguf",
    url: "https://huggingface.co/bartowski/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf",
    sha256: "6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653",
};

const WORKER_START_TIMEOUT: Duration = Duration::from_secs(45);
const WORKER_REQUEST_TIMEOUT: Duration = Duration::from_secs(7);

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
    runtime: Arc<Mutex<PluginRuntimeStatus>>,
    enabled: Arc<AtomicBool>,
}

impl Default for PromptEnhancer {
    fn default() -> Self {
        Self {
            worker: Arc::new(Mutex::new(None)),
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
        self.stop_worker();
        let mut child = Command::new(executable)
            .arg("--model")
            .arg(model)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| prompt_worker_spawn_error(executable, error))?;
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
        *self.worker.lock().expect("plugin worker mutex poisoned") = Some(WorkerHandle {
            child,
            stdin,
            responses,
            next_request_id: 1,
        });
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
        }
    }

    fn infer(
        &self,
        context: &PluginExecutionContext,
        settings: &BTreeMap<String, String>,
    ) -> Result<PluginExecutionOutput> {
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
            })) if response_id == request_id => Ok(PluginExecutionOutput { text, backend }),
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
        let result = self.infer(context, settings);
        if let Err(error) = &result {
            self.stop_worker();
            self.set_runtime_status(PluginRuntimeStatus {
                state: PluginRuntimeState::Error,
                downloaded_bytes: 0,
                total_bytes: None,
                message: Some(error.to_string()),
            });
        }
        result
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
        "<|im_start|>system\nRewrite the spoken request as a precise prompt for the specified coding model. Preserve the user's intent and all concrete requirements. Tailor the structure and level of detail for the target model. Add useful structure, acceptance criteria, and constraints only when they follow from the request. Do not invent technologies, files, facts, or requirements. Return only the enhanced prompt; never answer it.<|im_end|>\n<|im_start|>user\nTarget coding model: {}\nActive application: {}\nSpoken request:\n{}<|im_end|>\n<|im_start|>assistant\n",
        target_model_label(settings),
        context.active_application,
        context.current_text
    )
}

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn prompt_worker_spawn_error(executable: &Path, error: std::io::Error) -> anyhow::Error {
    #[cfg(windows)]
    if error.raw_os_error() == Some(4551) {
        return anyhow::anyhow!(
            "Windows Smart App Control blocked the prompt enhancer worker. Enable Developer Mode in Windows Settings, reboot Windows, then restart `npm run tauri:dev`"
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
        assert!(prompt.contains("Active application: Editor"));
        assert!(prompt.contains("never answer it"));
    }

    #[test]
    fn model_descriptor_uses_plugin_directory() {
        let installer = ModelInstaller::from_descriptor(Path::new("app-data"), MODEL_DESCRIPTOR);
        assert!(
            installer
                .model_path()
                .ends_with(Path::new("models/llama/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn explains_windows_application_control_spawn_failure() {
        let error = prompt_worker_spawn_error(
            Path::new("banshee-prompt-worker.exe"),
            std::io::Error::from_raw_os_error(4551),
        );
        assert!(error.to_string().contains("Windows Smart App Control"));
        assert!(error.to_string().contains("Enable Developer Mode"));
        assert!(error.to_string().contains("reboot Windows"));
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
