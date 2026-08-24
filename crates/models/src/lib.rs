//! Installation and readiness tracking for local speech models.

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const DEFAULT_MODEL_NAME: &str = "tiny.en-q5_1";
pub const DEFAULT_MODEL_FILE: &str = "ggml-tiny.en-q5_1.bin";
pub const DEFAULT_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en-q5_1.bin";
pub const DEFAULT_MODEL_SHA1: &str = "3fb92ec865cbbc769f08137f22470d6b66e071b6";

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
    pub state: ModelState,
    pub model_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub message: Option<String>,
}

impl ModelStatus {
    fn new(state: ModelState) -> Self {
        Self {
            state,
            model_name: DEFAULT_MODEL_NAME.to_string(),
            downloaded_bytes: 0,
            total_bytes: None,
            message: None,
        }
    }
}

#[derive(Clone)]
pub struct ModelInstaller {
    model_path: PathBuf,
    status: Arc<Mutex<ModelStatus>>,
    running: Arc<AtomicBool>,
}

impl ModelInstaller {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            model_path: data_dir
                .join("models")
                .join("whisper")
                .join(DEFAULT_MODEL_FILE),
            status: Arc::new(Mutex::new(ModelStatus::new(ModelState::Missing))),
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
                let mut status = ModelStatus::new(ModelState::Error);
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
        if !valid_model(&self.model_path)? {
            self.download(on_status)?;
        }
        self.publish(ModelStatus::new(ModelState::Loading), on_status);
        load_model(&self.model_path).context("failed to initialize the speech model")?;
        self.publish(ModelStatus::new(ModelState::Ready), on_status);
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
        let partial_path = self.model_path.with_extension("bin.part");
        let client = Client::builder()
            .timeout(Duration::from_secs(900))
            .build()?;
        let mut response = client
            .get(DEFAULT_MODEL_URL)
            .send()
            .context("failed to download the speech model")?
            .error_for_status()
            .context("speech model download returned an error")?;
        let total_bytes = response.content_length();
        let mut status = ModelStatus::new(ModelState::Downloading);
        status.total_bytes = total_bytes;
        self.publish(status.clone(), on_status);

        let mut output = File::create(&partial_path)?;
        let mut hasher = Sha1::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = response.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read])?;
            hasher.update(&buffer[..read]);
            status.downloaded_bytes += read as u64;
            self.publish(status.clone(), on_status);
        }
        output.sync_all()?;
        let digest = format!("{:x}", hasher.finalize());
        if digest != DEFAULT_MODEL_SHA1 {
            let _ = fs::remove_file(&partial_path);
            bail!("downloaded speech model failed integrity validation");
        }
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

fn valid_model(path: &Path) -> Result<bool> {
    if !path.is_file() {
        return Ok(false);
    }
    let mut file = File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()) == DEFAULT_MODEL_SHA1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_is_not_valid() {
        let path = std::env::temp_dir().join(format!("banshee-missing-{}", std::process::id()));
        assert!(!valid_model(&path).expect("validation should succeed"));
    }

    #[test]
    fn model_path_is_below_application_data() {
        let installer = ModelInstaller::new(Path::new("app-data"));
        assert!(
            installer
                .model_path()
                .ends_with(Path::new("models/whisper/ggml-tiny.en-q5_1.bin"))
        );
    }
}
