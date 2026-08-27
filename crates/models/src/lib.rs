//! Installation and readiness tracking for local inference models.

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const DEFAULT_MODEL_NAME: &str = "base.en";
pub const DEFAULT_MODEL_FILE: &str = "ggml-base.en.bin";
pub const DEFAULT_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";
pub const DEFAULT_MODEL_SHA256: &str =
    "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002";
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    Speech,
    Cleanup,
}

#[derive(Debug, Clone, Copy)]
pub struct ModelDescriptor {
    pub capability: ModelCapability,
    pub name: &'static str,
    pub directory: &'static str,
    pub file: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
}

const SPEECH_MODEL: ModelDescriptor = ModelDescriptor {
    capability: ModelCapability::Speech,
    name: DEFAULT_MODEL_NAME,
    directory: "whisper",
    file: DEFAULT_MODEL_FILE,
    url: DEFAULT_MODEL_URL,
    sha256: DEFAULT_MODEL_SHA256,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelState {
    Missing,
    Downloading,
    Loading,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub capability: ModelCapability,
    pub state: ModelState,
    pub model_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelsStatus {
    pub speech: ModelStatus,
    pub cleanup: ModelStatus,
}

impl ModelStatus {
    fn new(descriptor: ModelDescriptor, state: ModelState) -> Self {
        Self {
            capability: descriptor.capability,
            state,
            model_name: descriptor.name.to_string(),
            downloaded_bytes: 0,
            total_bytes: None,
            message: None,
        }
    }
}

#[derive(Clone)]
pub struct ModelInstaller {
    descriptor: ModelDescriptor,
    model_path: PathBuf,
    status: Arc<Mutex<ModelStatus>>,
    running: Arc<AtomicBool>,
}

impl ModelInstaller {
    pub fn new(data_dir: &Path) -> Self {
        Self::from_descriptor(data_dir, SPEECH_MODEL)
    }

    pub fn from_descriptor(data_dir: &Path, descriptor: ModelDescriptor) -> Self {
        Self {
            descriptor,
            model_path: data_dir
                .join("models")
                .join(descriptor.directory)
                .join(descriptor.file),
            status: Arc::new(Mutex::new(ModelStatus::new(
                descriptor,
                ModelState::Missing,
            ))),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn status(&self) -> ModelStatus {
        self.status
            .lock()
            .expect("model status mutex poisoned")
            .clone()
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    pub fn ensure_installed<F, L>(&self, on_status: F, load_model: L)
    where
        F: Fn(ModelStatus) + Send + 'static,
        L: Fn(&Path) -> Result<()> + Send + 'static,
    {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let installer = self.clone();
        std::thread::spawn(move || {
            let result = installer.install(&on_status, &load_model);
            if let Err(error) = result {
                let mut status = ModelStatus::new(installer.descriptor, ModelState::Error);
                status.message = Some(error.to_string());
                installer.publish(status, &on_status);
            }
            installer.running.store(false, Ordering::SeqCst);
        });
    }

    fn install<F, L>(&self, on_status: &F, load_model: &L) -> Result<()>
    where
        F: Fn(ModelStatus),
        L: Fn(&Path) -> Result<()>,
    {
        if !valid_model(&self.model_path, self.descriptor.sha256)? {
            self.download(on_status)?;
        }
        self.publish(
            ModelStatus::new(self.descriptor, ModelState::Loading),
            on_status,
        );
        load_model(&self.model_path).context("failed to initialize the model")?;
        self.publish(
            ModelStatus::new(self.descriptor, ModelState::Ready),
            on_status,
        );
        Ok(())
    }

    fn download<F>(&self, on_status: &F) -> Result<()>
    where
        F: Fn(ModelStatus),
    {
        let parent = self
            .model_path
            .parent()
            .context("model path has no parent")?;
        fs::create_dir_all(parent)?;
        let partial_path = self.model_path.with_extension("part");
        let client = Client::builder()
            .timeout(Duration::from_secs(900))
            .build()?;
        let mut response = client
            .get(self.descriptor.url)
            .send()
            .context("failed to download the model")?
            .error_for_status()
            .context("model download returned an error")?;
        let total_bytes = response.content_length();
        let mut status = ModelStatus::new(self.descriptor, ModelState::Downloading);
        status.total_bytes = total_bytes;
        self.publish(status.clone(), on_status);

        let mut output = File::create(&partial_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        let mut last_published_bytes = 0_u64;
        loop {
            let read = response.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read])?;
            hasher.update(&buffer[..read]);
            status.downloaded_bytes += read as u64;
            if status.downloaded_bytes.saturating_sub(last_published_bytes) >= 1024 * 1024 {
                self.publish(status.clone(), on_status);
                last_published_bytes = status.downloaded_bytes;
            }
        }
        self.publish(status, on_status);
        output.sync_all()?;
        let digest = format!("{:x}", hasher.finalize());
        validate_download(
            &partial_path,
            self.descriptor.name,
            &digest,
            self.descriptor.sha256,
        )?;
        if self.model_path.exists() {
            fs::remove_file(&self.model_path)?;
        }
        fs::rename(partial_path, &self.model_path)?;
        Ok(())
    }

    fn publish<F>(&self, status: ModelStatus, on_status: &F)
    where
        F: Fn(ModelStatus),
    {
        *self.status.lock().expect("model status mutex poisoned") = status.clone();
        on_status(status);
    }
}

fn validate_download(path: &Path, model_name: &str, actual: &str, expected: &str) -> Result<()> {
    if actual == expected {
        return Ok(());
    }

    let _ = fs::remove_file(path);
    bail!(
        "downloaded model {model_name} failed integrity validation: expected SHA-256 {expected}, got {actual}"
    );
}

fn valid_model(path: &Path, expected: &str) -> Result<bool> {
    if !path.is_file() {
        return Ok(false);
    }
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()) == expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("banshee-{name}-{}", std::process::id()))
    }

    #[test]
    fn missing_model_is_not_valid() {
        let path = temp_file("missing");
        assert!(!valid_model(&path, DEFAULT_MODEL_SHA256).expect("validation should succeed"));
    }

    #[test]
    fn model_is_valid_only_when_file_hash_matches() {
        let path = temp_file("valid-model");
        fs::write(&path, b"test model").expect("test model should be written");

        assert!(
            valid_model(
                &path,
                "e6a8036e9cb9f8c9f52ffd985c4d6f0498d94e6919f4c67b6423da0176e549fa"
            )
            .expect("validation should succeed")
        );
        assert!(!valid_model(&path, DEFAULT_MODEL_SHA256).expect("validation should succeed"));

        fs::remove_file(path).expect("test model should be removed");
    }

    #[test]
    fn rejected_download_is_removed_and_reports_digests() {
        let path = temp_file("invalid-download.part");
        fs::write(&path, b"invalid model").expect("partial model should be written");

        let error = validate_download(&path, "cleanup-model", "actual", "expected")
            .expect_err("mismatched download should be rejected")
            .to_string();

        assert!(!path.exists());
        assert!(error.contains("cleanup-model"));
        assert!(error.contains("expected SHA-256 expected, got actual"));
    }

    #[test]
    fn model_path_is_below_application_data() {
        let installer = ModelInstaller::new(Path::new("app-data"));
        assert!(
            installer
                .model_path()
                .ends_with(Path::new("models/whisper/ggml-base.en.bin"))
        );
    }
}
