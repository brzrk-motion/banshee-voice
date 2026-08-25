use anyhow::{Context, Result, bail};
use banshee_contracts::domain::PluginExecutionContext;
use banshee_prompt_enhancer::{
    WORKER_PROTOCOL_VERSION, WorkerRequest, WorkerResponse, enhancement_prompt,
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

const INFERENCE_DEADLINE: Duration = Duration::from_secs(5);
const BACKEND: &str = "llama.cpp:qwen2.5-0.5b-q4_k_m:cpu";

struct PromptModel {
    model: LlamaModel,
    backend: LlamaBackend,
}

impl PromptModel {
    fn load(path: &Path) -> Result<Self> {
        let backend = LlamaBackend::init().context("failed to initialize llama.cpp")?;
        let model = LlamaModel::load_from_file(&backend, path, &LlamaModelParams::default())
            .context("failed to load prompt enhancer model")?;
        Ok(Self { model, backend })
    }

    fn infer(
        &self,
        context: &PluginExecutionContext,
        settings: &BTreeMap<String, String>,
    ) -> Result<String> {
        let started = Instant::now();
        let prompt = enhancement_prompt(context, settings);
        let params = LlamaContextParams::default()
            .with_n_ctx(Some(NonZeroU32::new(4096).expect("nonzero context")))
            .with_n_threads(8)
            .with_n_threads_batch(8);
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
        for position in first..first + 768 {
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
        Ok(output.trim().trim_matches('"').trim().to_string())
    }
}

fn main() -> Result<()> {
    let model_path = parse_model_path()?;
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    let model = match PromptModel::load(&model_path) {
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
                backend: BACKEND.into(),
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

fn parse_model_path() -> Result<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    match (args.next().as_deref(), args.next(), args.next()) {
        (Some(flag), Some(path), None) if flag == "--model" => Ok(path.into()),
        _ => bail!("usage: banshee-prompt-worker --model <path>"),
    }
}

fn write_response(writer: &mut impl Write, response: &WorkerResponse) -> Result<()> {
    serde_json::to_writer(&mut *writer, response)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}
