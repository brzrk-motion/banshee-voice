//! Generic compiled-in text transformation plugin registry and ordered runtime.

use anyhow::{Result, bail};
use banshee_contracts::domain::{
    PluginExecutionContext, PluginManifest, PluginPipelineOutput, PluginRunRecord, PluginRunStatus,
    PluginRunner, PluginRuntimeState, PluginSettingControl, PluginStateStore, PluginSummary,
    TextTransformPlugin,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

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
                let settings = resolve_settings(&manifest, &self.state.settings(&manifest.id)?);
                Ok(PluginSummary {
                    enabled: self.state.enabled(&manifest.id)?,
                    manifest,
                    settings,
                    runtime_state: runtime.state,
                    downloaded_bytes: runtime.downloaded_bytes,
                    total_bytes: runtime.total_bytes,
                    message: runtime.message,
                })
            })
            .collect()
    }

    pub fn set_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
        self.plugin(plugin_id)?;
        self.state.set_enabled(plugin_id, enabled)
    }

    pub fn set_settings(&self, plugin_id: &str, settings: BTreeMap<String, String>) -> Result<()> {
        let manifest = self.plugin(plugin_id)?.manifest();
        for (key, value) in &settings {
            let definition = manifest
                .settings
                .iter()
                .find(|definition| definition.key == *key)
                .ok_or_else(|| anyhow::anyhow!("unknown setting for {plugin_id}: {key}"))?;
            match &definition.control {
                PluginSettingControl::Select { options, .. }
                    if !options.iter().any(|option| option.value == *value) =>
                {
                    bail!("invalid value for {plugin_id}.{key}: {value}");
                }
                PluginSettingControl::Select { .. } => {}
            }
        }
        let canonical = resolve_settings(&manifest, &settings);
        self.state.set_settings(plugin_id, &canonical)
    }

    fn plugin(&self, plugin_id: &str) -> Result<&Arc<dyn TextTransformPlugin>> {
        self.plugins
            .iter()
            .find(|plugin| plugin.manifest().id == plugin_id)
            .ok_or_else(|| anyhow::anyhow!("unknown plugin: {plugin_id}"))
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
            let settings = resolve_settings(&manifest, &self.state.settings(&manifest.id)?);
            match plugin.transform(&context, &settings) {
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

fn resolve_settings(
    manifest: &PluginManifest,
    stored: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    manifest
        .settings
        .iter()
        .map(|definition| {
            let value = match &definition.control {
                PluginSettingControl::Select {
                    default_value,
                    options,
                } => stored
                    .get(&definition.key)
                    .filter(|value| {
                        options
                            .iter()
                            .any(|option| option.value.as_str() == value.as_str())
                    })
                    .cloned()
                    .unwrap_or_else(|| default_value.clone()),
            };
            (definition.key.clone(), value)
        })
        .collect()
}

fn valid_output(output: &str) -> bool {
    !output.trim().is_empty() && output.chars().count() <= 16_000
}

#[cfg(test)]
mod tests {
    use super::*;
    use banshee_contracts::domain::{
        PluginExecutionOutput, PluginRuntimeStatus, PluginSettingDefinition, PluginSettingOption,
        ProfileSummary, RecordingOrigin,
    };
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryState {
        enabled: Mutex<BTreeMap<String, bool>>,
        settings: Mutex<BTreeMap<String, BTreeMap<String, String>>>,
    }

    impl PluginStateStore for MemoryState {
        fn enabled(&self, id: &str) -> Result<bool> {
            Ok(*self.enabled.lock().unwrap().get(id).unwrap_or(&false))
        }

        fn set_enabled(&self, id: &str, enabled: bool) -> Result<()> {
            self.enabled.lock().unwrap().insert(id.into(), enabled);
            Ok(())
        }

        fn settings(&self, id: &str) -> Result<BTreeMap<String, String>> {
            Ok(self
                .settings
                .lock()
                .unwrap()
                .get(id)
                .cloned()
                .unwrap_or_default())
        }

        fn set_settings(&self, id: &str, settings: &BTreeMap<String, String>) -> Result<()> {
            self.settings
                .lock()
                .unwrap()
                .insert(id.into(), settings.clone());
            Ok(())
        }
    }

    struct TestPlugin {
        id: &'static str,
        suffix: &'static str,
        state: PluginRuntimeState,
        configurable: bool,
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
                settings: self
                    .configurable
                    .then(|| PluginSettingDefinition {
                        key: "target".into(),
                        label: "Target".into(),
                        description: None,
                        control: PluginSettingControl::Select {
                            default_value: "one".into(),
                            options: vec![
                                PluginSettingOption {
                                    value: "one".into(),
                                    label: "One".into(),
                                },
                                PluginSettingOption {
                                    value: "two".into(),
                                    label: "Two".into(),
                                },
                            ],
                        },
                    })
                    .into_iter()
                    .collect(),
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

        fn transform(
            &self,
            context: &PluginExecutionContext,
            settings: &BTreeMap<String, String>,
        ) -> Result<PluginExecutionOutput> {
            let target = settings
                .get("target")
                .map(|value| format!("-{value}"))
                .unwrap_or_default();
            Ok(PluginExecutionOutput {
                text: format!("{}{}{}", context.current_text, self.suffix, target),
                backend: "test".into(),
            })
        }
    }

    fn plugin(id: &'static str, suffix: &'static str) -> Arc<TestPlugin> {
        Arc::new(TestPlugin {
            id,
            suffix,
            state: PluginRuntimeState::Ready,
            configurable: false,
        })
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
    fn plugins_are_disabled_by_default() {
        let registry =
            PluginRegistry::new(Arc::new(MemoryState::default()), vec![plugin("one", "")]);
        assert!(!registry.list().unwrap()[0].enabled);
    }

    #[test]
    fn runs_enabled_plugins_in_registry_order() {
        let state = Arc::new(MemoryState::default());
        state.set_enabled("one", true).unwrap();
        state.set_enabled("two", true).unwrap();
        let registry =
            PluginRegistry::new(state, vec![plugin("one", "-one"), plugin("two", "-two")]);
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
                configurable: false,
            })],
        );
        let result = registry.run(context()).unwrap();
        assert_eq!(result.final_text, "clean");
        assert_eq!(result.runs[0].status, PluginRunStatus::Skipped);
    }

    #[test]
    fn resolves_defaults_and_passes_saved_settings_to_transform() {
        let state = Arc::new(MemoryState::default());
        state.set_enabled("configurable", true).unwrap();
        let registry = PluginRegistry::new(
            state,
            vec![Arc::new(TestPlugin {
                id: "configurable",
                suffix: "",
                state: PluginRuntimeState::Ready,
                configurable: true,
            })],
        );

        assert_eq!(registry.list().unwrap()[0].settings["target"], "one");
        registry
            .set_settings(
                "configurable",
                BTreeMap::from([("target".into(), "two".into())]),
            )
            .unwrap();
        assert_eq!(registry.run(context()).unwrap().final_text, "clean-two");
    }

    #[test]
    fn rejects_unknown_keys_and_invalid_select_values() {
        let registry = PluginRegistry::new(
            Arc::new(MemoryState::default()),
            vec![Arc::new(TestPlugin {
                id: "configurable",
                suffix: "",
                state: PluginRuntimeState::Ready,
                configurable: true,
            })],
        );

        assert!(
            registry
                .set_settings(
                    "configurable",
                    BTreeMap::from([("unknown".into(), "two".into())]),
                )
                .is_err()
        );
        assert!(
            registry
                .set_settings(
                    "configurable",
                    BTreeMap::from([("target".into(), "three".into())]),
                )
                .is_err()
        );
    }
}
