use anyhow::{Context, Result, bail};
use banshee_contracts::domain::{
    AccelerationBackend, AccelerationPreference, PluginExecutionContext,
};
use banshee_prompt_enhancer::{
    WORKER_PROTOCOL_VERSION, WorkerRequest, WorkerResponse, enhancement_prompt,
    sanitize_enhancement,
};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const INFERENCE_DEADLINE: Duration = Duration::from_secs(12);
const MAX_OUTPUT_TOKENS: i32 = 192;
const CONTEXT_TOKENS: u32 = 2048;
const MAX_THREADS: usize = 4;
const BACKEND_PREFIX: &str = "llama.cpp:nemotron3-nano-4b-q4_k_m";

struct PromptModel {
    model: LlamaModel,
    backend: LlamaBackend,
    acceleration: AccelerationBackend,
}

impl PromptModel {
    fn load(path: &Path, preference: AccelerationPreference) -> Result<Self> {
        let backend = LlamaBackend::init().context("failed to initialize llama.cpp")?;
        let gpu_available = backend.supports_gpu_offload();
        let load_cpu = || {
            LlamaModel::load_from_file(
                &backend,
                path,
                &LlamaModelParams::default().with_n_gpu_layers(0),
            )
        };
        let load_gpu = || LlamaModel::load_from_file(&backend, path, &LlamaModelParams::default());

        let (model, acceleration) = match preference {
            AccelerationPreference::Cpu => (load_cpu()?, AccelerationBackend::Cpu),
            AccelerationPreference::Gpu => {
                if !gpu_available {
                    bail!("Vulkan GPU acceleration is unavailable");
                }
                (load_gpu()?, AccelerationBackend::Gpu)
            }
            AccelerationPreference::Auto if gpu_available => match load_gpu() {
                Ok(model) => (model, AccelerationBackend::Gpu),
                Err(_) => (load_cpu()?, AccelerationBackend::Cpu),
            },
            AccelerationPreference::Auto => (load_cpu()?, AccelerationBackend::Cpu),
        };
        Ok(Self {
            model,
            backend,
            acceleration,
        })
    }

    fn infer(
        &self,
        context: &PluginExecutionContext,
        settings: &BTreeMap<String, String>,
    ) -> Result<String> {
        let started = Instant::now();
        let prompt = enhancement_prompt(context, settings);
        let threads = worker_threads() as i32;
        let params = LlamaContextParams::default()
            .with_n_ctx(Some(
                NonZeroU32::new(CONTEXT_TOKENS).expect("nonzero context"),
            ))
            .with_n_threads(threads)
            .with_n_threads_batch(threads);
        let mut llama = self.model.new_context(&self.backend, params)?;
        let tokens = self.model.str_to_token(&prompt, AddBos::Always)?;
        if tokens.len() + 16 >= llama.n_ctx() as usize {
            bail!("plugin prompt exceeds model context");
        }
        let mut batch = LlamaBatch::new(tokens.len().max(1), 1);
        let last = tokens.len() as i32 - 1;
        for (position, token) in (0_i32..).zip(tokens) {
            batch.add(token, position, &[0], position == last)?;
        }
        llama.decode(&mut batch)?;
        let mut sampler = LlamaSampler::greedy();
        let mut output = String::new();
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let first = batch.n_tokens();
        for position in first..first + MAX_OUTPUT_TOKENS {
            if started.elapsed() >= INFERENCE_DEADLINE {
                bail!("prompt enhancement timed out");
            }
            let token = sampler.sample(&llama, batch.n_tokens() - 1);
            sampler.accept(token);
            if self.model.is_eog_token(token) {
                break;
            }
            output.push_str(
                &self
                    .model
                    .token_to_piece(token, &mut decoder, false, None)?,
            );
            batch.clear();
            batch.add(token, position, &[0], true)?;
            llama.decode(&mut batch)?;
        }
        Ok(sanitize_enhancement(&output))
    }
}

fn main() -> Result<()> {
    let args = parse_args(std::env::args_os().skip(1))?;
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    let model = match PromptModel::load(&args.model_path, args.acceleration) {
        Ok(model) => model,
        Err(error) => {
            write_response(
                &mut writer,
                &WorkerResponse::Error {
                    request_id: None,
                    message: format!("{error:#}"),
                },
            )?;
            return Err(error);
        }
    };
    write_response(
        &mut writer,
        &WorkerResponse::Ready {
            protocol_version: WORKER_PROTOCOL_VERSION,
        },
    )?;

    for line in BufReader::new(std::io::stdin().lock()).lines() {
        let line = line?;
        let request = match serde_json::from_str::<WorkerRequest>(&line) {
            Ok(request) => request,
            Err(error) => {
                write_response(
                    &mut writer,
                    &WorkerResponse::Error {
                        request_id: None,
                        message: format!("invalid worker request: {error}"),
                    },
                )?;
                continue;
            }
        };
        if request.protocol_version != WORKER_PROTOCOL_VERSION {
            write_response(
                &mut writer,
                &WorkerResponse::Error {
                    request_id: Some(request.request_id),
                    message: format!(
                        "unsupported worker protocol version: {}",
                        request.protocol_version
                    ),
                },
            )?;
            continue;
        }
        let response = match model.infer(&request.context, &request.settings) {
            Ok(text) => WorkerResponse::Transformed {
                request_id: request.request_id,
                text,
                backend: format!("{BACKEND_PREFIX}:{}", model.acceleration.as_str()),
            },
            Err(error) => WorkerResponse::Error {
                request_id: Some(request.request_id),
                message: error.to_string(),
            },
        };
        write_response(&mut writer, &response)?;
    }
    Ok(())
}

#[derive(Debug)]
struct WorkerArgs {
    model_path: PathBuf,
    acceleration: AccelerationPreference,
}

fn parse_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<WorkerArgs> {
    let mut args = args.into_iter();
    match (
        args.next().as_deref(),
        args.next(),
        args.next().as_deref(),
        args.next().and_then(|value| value.into_string().ok()),
        args.next(),
    ) {
        (Some(model_flag), Some(model_path), Some(acceleration_flag), Some(acceleration), None)
            if model_flag == "--model" && acceleration_flag == "--acceleration" =>
        {
            let acceleration = match acceleration.as_str() {
                "auto" => AccelerationPreference::Auto,
                "cpu" => AccelerationPreference::Cpu,
                "gpu" => AccelerationPreference::Gpu,
                _ => bail!("acceleration must be auto, cpu, or gpu"),
            };
            Ok(WorkerArgs {
                model_path: model_path.into(),
                acceleration,
            })
        }
        _ => bail!("usage: banshee-prompt-worker --model <path> --acceleration <auto|cpu|gpu>"),
    }
}

fn write_response(writer: &mut impl Write, response: &WorkerResponse) -> Result<()> {
    serde_json::to_writer(&mut *writer, response)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn worker_threads() -> usize {
    std::thread::available_parallelism()
        .map(|threads| threads.get())
        .unwrap_or(MAX_THREADS)
        .min(MAX_THREADS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_acceleration_argument() {
        let args = parse_args([
            "--model".into(),
            "model.gguf".into(),
            "--acceleration".into(),
            "gpu".into(),
        ])
        .expect("worker arguments should parse");
        assert_eq!(args.model_path, Path::new("model.gguf"));
        assert_eq!(args.acceleration, AccelerationPreference::Gpu);
    }

    #[test]
    fn rejects_unknown_acceleration() {
        let error = parse_args([
            "--model".into(),
            "model.gguf".into(),
            "--acceleration".into(),
            "magic".into(),
        ])
        .expect_err("unknown acceleration should fail");
        assert!(error.to_string().contains("auto, cpu, or gpu"));
    }
}
