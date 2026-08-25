//! Compiled-in text transformation plugins and their ordered runtime.

use anyhow::{Context, Result, bail};
use banshee_contracts::domain::{
    PluginExecutionContext, PluginExecutionOutput, PluginManifest, PluginPipelineOutput,
    PluginRunRecord, PluginRunStatus, PluginRunner, PluginRuntimeState, PluginRuntimeStatus,
    PluginStateStore, PluginSummary, TextTransformPlugin,
};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const PROMPT_ENHANCER_ID: &str = "banshee.prompt-enhancer";
pub const WORKER_PROTOCOL_VERSION: u32 = 1;
const WORKER_START_TIMEOUT: Duration = Duration::from_secs(45);
const WORKER_REQUEST_TIMEOUT: Duration = Duration::from_secs(7);

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkerRequest {
    pub protocol_version: u32,
    pub request_id: u64,
    pub context: PluginExecutionContext,
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

pub struct PluginRegistry {
    plugins: Vec<Arc<dyn TextTransformPlugin>>,
    state: Arc<dyn PluginStateStore>,
}

impl PluginRegistry {
    pub fn new(
        state: Arc<dyn PluginStateStore>,
        plugins: Vec<Arc<dyn TextTransformPlugin>>,
    ) -> Self {
        Self { plugins, state }
    }

    pub fn list(&self) -> Result<Vec<PluginSummary>> {
        self.plugins
            .iter()
            .map(|plugin| {
                let manifest = plugin.manifest();
                let runtime = plugin.runtime_status();
                Ok(PluginSummary {
                    enabled: self.state.enabled(&manifest.id)?,
                    manifest,
                    runtime_state: runtime.state,
                    downloaded_bytes: runtime.downloaded_bytes,
                    total_bytes: runtime.total_bytes,
                    message: runtime.message,
                })
            })
            .collect()
    }

    pub fn set_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
        if !self
            .plugins
            .iter()
            .any(|plugin| plugin.manifest().id == plugin_id)
        {
            bail!("unknown plugin: {plugin_id}");
        }
        self.state.set_enabled(plugin_id, enabled)
    }
}

impl PluginRunner for PluginRegistry {
    fn run(&self, mut context: PluginExecutionContext) -> Result<PluginPipelineOutput> {
        let mut runs = Vec::new();
        for plugin in &self.plugins {
            let manifest = plugin.manifest();
            if !self.state.enabled(&manifest.id)? {
                continue;
            }
            let started = Instant::now();
            let runtime = plugin.runtime_status();
            if runtime.state != PluginRuntimeState::Ready {
                runs.push(PluginRunRecord {
                    plugin_id: manifest.id,
                    status: PluginRunStatus::Skipped,
                    latency_ms: started.elapsed().as_millis() as u64,
                    backend: None,
                    fallback_reason: Some(
                        runtime
                            .message
                            .unwrap_or_else(|| "plugin is not ready".into()),
                    ),
                });
                continue;
            }
            match plugin.transform(&context) {
                Ok(output) if valid_output(&output.text) => {
                    context.current_text = output.text;
                    runs.push(PluginRunRecord {
                        plugin_id: manifest.id,
                        status: PluginRunStatus::Applied,
                        latency_ms: started.elapsed().as_millis() as u64,
                        backend: Some(output.backend),
                        fallback_reason: None,
                    });
                }
                Ok(_) => runs.push(PluginRunRecord {
                    plugin_id: manifest.id,
                    status: PluginRunStatus::Failed,
                    latency_ms: started.elapsed().as_millis() as u64,
                    backend: None,
                    fallback_reason: Some("plugin returned invalid output".into()),
                }),
                Err(error) => runs.push(PluginRunRecord {
                    plugin_id: manifest.id,
                    status: PluginRunStatus::Failed,
                    latency_ms: started.elapsed().as_millis() as u64,
                    backend: None,
                    fallback_reason: Some(error.to_string()),
                }),
            }
        }
        Ok(PluginPipelineOutput {
            final_text: context.current_text,
            runs,
        })
    }
}

fn valid_output(output: &str) -> bool {
    !output.trim().is_empty() && output.chars().count() <= 16_000
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
            .with_context(|| {
                format!(
                    "failed to start prompt enhancer worker at {}",
                    executable.display()
                )
            })?;
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

    fn infer(&self, context: &PluginExecutionContext) -> Result<PluginExecutionOutput> {
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
        }
    }

    fn runtime_status(&self) -> PluginRuntimeStatus {
        self.runtime
            .lock()
            .expect("plugin status mutex poisoned")
            .clone()
    }

    fn transform(&self, context: &PluginExecutionContext) -> Result<PluginExecutionOutput> {
        let result = self.infer(context);
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

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;
    use banshee_contracts::domain::{ProfileSummary, RecordingOrigin};
    use std::collections::HashMap;

    #[derive(Default)]
    struct MemoryState(Mutex<HashMap<String, bool>>);
    impl PluginStateStore for MemoryState {
        fn enabled(&self, id: &str) -> Result<bool> {
            Ok(*self.0.lock().unwrap().get(id).unwrap_or(&false))
        }
        fn set_enabled(&self, id: &str, enabled: bool) -> Result<()> {
            self.0.lock().unwrap().insert(id.into(), enabled);
            Ok(())
        }
    }

    #[test]
    fn prompt_enhancer_is_disabled_by_default() {
        let registry = PluginRegistry::new(
            Arc::new(MemoryState::default()),
            vec![Arc::new(PromptEnhancer::default())],
        );
        assert!(!registry.list().unwrap()[0].enabled);
    }

    struct TestPlugin {
        id: &'static str,
        suffix: &'static str,
        state: PluginRuntimeState,
    }

    impl TextTransformPlugin for TestPlugin {
        fn manifest(&self) -> PluginManifest {
            PluginManifest {
                id: self.id.into(),
                name: self.id.into(),
                description: String::new(),
                version: "1".into(),
                author: "test".into(),
                stage: "test".into(),
            }
        }

        fn runtime_status(&self) -> PluginRuntimeStatus {
            PluginRuntimeStatus {
                state: self.state,
                downloaded_bytes: 0,
                total_bytes: None,
                message: None,
            }
        }

        fn transform(&self, context: &PluginExecutionContext) -> Result<PluginExecutionOutput> {
            Ok(PluginExecutionOutput {
                text: format!("{}{}", context.current_text, self.suffix),
                backend: "test".into(),
            })
        }
    }

    fn context() -> PluginExecutionContext {
        PluginExecutionContext {
            raw_text: "raw".into(),
            cleaned_text: "clean".into(),
            current_text: "clean".into(),
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
    fn runs_enabled_plugins_in_registry_order() {
        let state = Arc::new(MemoryState::default());
        state.set_enabled("one", true).unwrap();
        state.set_enabled("two", true).unwrap();
        let registry = PluginRegistry::new(
            state,
            vec![
                Arc::new(TestPlugin {
                    id: "one",
                    suffix: "-one",
                    state: PluginRuntimeState::Ready,
                }),
                Arc::new(TestPlugin {
                    id: "two",
                    suffix: "-two",
                    state: PluginRuntimeState::Ready,
                }),
            ],
        );
        let result = registry.run(context()).unwrap();
        assert_eq!(result.final_text, "clean-one-two");
        assert!(
            result
                .runs
                .iter()
                .all(|run| run.status == PluginRunStatus::Applied)
        );
    }

    #[test]
    fn unready_plugin_preserves_cleaned_text() {
        let state = Arc::new(MemoryState::default());
        state.set_enabled("waiting", true).unwrap();
        let registry = PluginRegistry::new(
            state,
            vec![Arc::new(TestPlugin {
                id: "waiting",
                suffix: "-changed",
                state: PluginRuntimeState::Downloading,
            })],
        );
        let result = registry.run(context()).unwrap();
        assert_eq!(result.final_text, "clean");
        assert_eq!(result.runs[0].status, PluginRunStatus::Skipped);
    }

    #[test]
    fn worker_protocol_uses_versioned_camel_case_json() {
        let request = WorkerRequest {
            protocol_version: WORKER_PROTOCOL_VERSION,
            request_id: 42,
            context: context(),
        };
        let json = serde_json::to_value(request).unwrap();
        assert_eq!(json["protocolVersion"], 1);
        assert_eq!(json["requestId"], 42);
        assert_eq!(json["context"]["currentText"], "clean");

        let ready = serde_json::to_value(WorkerResponse::Ready {
            protocol_version: WORKER_PROTOCOL_VERSION,
        })
        .unwrap();
        assert_eq!(
            ready,
            serde_json::json!({ "type": "ready", "protocolVersion": 1 })
        );
    }

    #[cfg(unix)]
    #[test]
    fn prompt_enhancer_communicates_with_and_stops_worker() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let script =
            std::env::temp_dir().join(format!("banshee-fake-prompt-worker-{}", std::process::id()));
        fs::write(
            &script,
            "#!/bin/sh\necho '{\"type\":\"ready\",\"protocolVersion\":1}'\nIFS= read -r request\necho '{\"type\":\"transformed\",\"requestId\":1,\"text\":\"enhanced\",\"backend\":\"fake\"}'\n",
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
        let output = enhancer.transform(&context()).unwrap();
        assert_eq!(output.text, "enhanced");
        assert_eq!(output.backend, "fake");

        enhancer.unload();
        assert_eq!(enhancer.runtime_status().state, PluginRuntimeState::Missing);
        fs::remove_file(script).unwrap();
    }
}
