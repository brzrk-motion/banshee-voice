use crate::{
    app_state::{
        ManagedAppState,
        ipc::{AppErrorDto, RecordingResultDto},
    },
    events::hud::{emit_audio_level, emit_hud_state, emit_recording_state},
};
use banshee_audio::AudioError;
use banshee_core::{
    domain::{
        AppError, AppErrorCode, AudioLevelChanged, FallbackUsed, HudState, HudStateChanged,
        OutputMethod, RecordingOrigin, RecordingState, RecordingStateChanged,
    },
    pipeline::PipelineError,
};
use banshee_injector::InjectorError;
use banshee_stt::SttError;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

fn map_pipeline_error(error: anyhow::Error) -> AppError {
    if let Some(audio_error) = error.downcast_ref::<AudioError>() {
        return AppError {
            code: match audio_error {
                AudioError::MicrophoneUnavailable | AudioError::StreamFailed(_) => {
                    AppErrorCode::MicrophoneUnavailable
                }
                AudioError::PermissionDenied => AppErrorCode::MicrophonePermissionDenied,
            },
            message: audio_error.to_string(),
            recoverable: true,
            fallback_used: Some(FallbackUsed::None),
        };
    }
    if let Some(stt_error) = error.downcast_ref::<SttError>() {
        return AppError {
            code: stt_error.code(),
            message: stt_error.to_string(),
            recoverable: true,
            fallback_used: Some(FallbackUsed::None),
        };
    }
    if error.downcast_ref::<PipelineError>().is_some() {
        return AppError {
            code: AppErrorCode::NoSpeechDetected,
            message: error.to_string(),
            recoverable: true,
            fallback_used: Some(FallbackUsed::None),
        };
    }
    if error.downcast_ref::<InjectorError>().is_some() {
        return AppError {
            code: AppErrorCode::ClipboardFailed,
            message: error.to_string(),
            recoverable: true,
            fallback_used: Some(FallbackUsed::None),
        };
    }
    AppError {
        code: AppErrorCode::Unknown,
        message: error.to_string(),
        recoverable: false,
        fallback_used: Some(FallbackUsed::None),
    }
}

fn transient_session_id() -> String {
    format!(
        "hud-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    )
}

fn schedule_hud_hide(app: tauri::AppHandle, session_id: String, delay_ms: u64) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        let state = app.state::<ManagedAppState>();
        let should_hide = {
            let mut recording = state.recording().lock().expect("recording mutex poisoned");
            if recording.snapshot.hud.session_id.as_deref() == Some(session_id.as_str()) {
                recording.snapshot.hud = HudStateChanged {
                    session_id: Some(session_id.clone()),
                    state: HudState::Hidden,
                    message: None,
                };
                true
            } else {
                false
            }
        };
        if should_hide {
            let _ = emit_hud_state(
                &app,
                HudStateChanged {
                    session_id: Some(session_id),
                    state: HudState::Hidden,
                    message: None,
                },
                None,
            );
        }
    });
}

fn show_shortcut_error(app: &tauri::AppHandle, state: &ManagedAppState, message: String) {
    let session_id = transient_session_id();
    let payload = HudStateChanged {
        session_id: Some(session_id.clone()),
        state: HudState::Error,
        message: Some(message),
    };
    state
        .recording()
        .lock()
        .expect("recording mutex poisoned")
        .snapshot
        .hud = payload.clone();
    let _ = emit_hud_state(app, payload, None);
    schedule_hud_hide(app.clone(), session_id, 2_500);
}

fn start_level_pump(app: tauri::AppHandle, session_id: String) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(33));
        loop {
            interval.tick().await;
            let state = app.state::<ManagedAppState>();
            let session = state
                .recording()
                .lock()
                .expect("recording mutex poisoned")
                .active_session
                .as_ref()
                .filter(|session| session.capture.id == session_id)
                .cloned();
            let Some(session) = session else {
                break;
            };
            let level = state
                .services()
                .recording_pipeline()
                .current_level(&session)
                .unwrap_or(0.0);
            let _ = emit_audio_level(
                &app,
                AudioLevelChanged {
                    session_id: session_id.clone(),
                    level,
                },
            );
        }
    });
}

pub fn start_recording_with_origin(
    app: &tauri::AppHandle,
    state: &ManagedAppState,
    origin: RecordingOrigin,
) -> Result<(), AppErrorDto> {
    if !state.model_ready() {
        let error = AppError {
            code: AppErrorCode::ModelMissing,
            message: "The speech model is still downloading or loading.".to_string(),
            recoverable: true,
            fallback_used: Some(FallbackUsed::None),
        };
        if origin == RecordingOrigin::PushToTalk {
            show_shortcut_error(app, state, error.message.clone());
        }
        return Err(error.into());
    }
    if state
        .recording()
        .lock()
        .expect("recording mutex poisoned")
        .active_session
        .is_some()
    {
        return Err(AppErrorDto::unknown(
            "a recording session is already active",
        ));
    }

    let session = match state.services().recording_pipeline().start(origin) {
        Ok(session) => session,
        Err(error) => {
            let error = map_pipeline_error(error);
            if origin == RecordingOrigin::PushToTalk {
                show_shortcut_error(app, state, error.message.clone());
            }
            return Err(error.into());
        }
    };
    let session_id = session.capture.id.clone();
    let target_bounds = session
        .output_target
        .as_ref()
        .and_then(|target| target.bounds);

    {
        let mut recording = state.recording().lock().expect("recording mutex poisoned");
        recording
            .begin_session(session)
            .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
        recording.snapshot.state = RecordingState::Recording;
        recording.snapshot.hud = if origin == RecordingOrigin::PushToTalk {
            HudStateChanged {
                session_id: Some(session_id.clone()),
                state: HudState::Recording,
                message: None,
            }
        } else {
            HudStateChanged {
                session_id: None,
                state: HudState::Hidden,
                message: None,
            }
        };
        recording.snapshot.last_error = None;
    }

    emit_recording_state(
        app,
        RecordingStateChanged {
            state: RecordingState::Recording,
            transcription_id: None,
        },
    )
    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
    if origin == RecordingOrigin::PushToTalk {
        emit_hud_state(
            app,
            HudStateChanged {
                session_id: Some(session_id.clone()),
                state: HudState::Recording,
                message: None,
            },
            target_bounds,
        )
        .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
        start_level_pump(app.clone(), session_id);
    }
    Ok(())
}

pub fn stop_recording(
    app: &tauri::AppHandle,
    state: &ManagedAppState,
) -> Result<RecordingResultDto, AppErrorDto> {
    let (origin, session_id, target_bounds) = {
        let mut recording = state.recording().lock().expect("recording mutex poisoned");
        let Some(session) = recording.active_session.as_ref() else {
            return Err(AppErrorDto::unknown("no active recording session"));
        };
        let values = (
            session.origin,
            session.capture.id.clone(),
            session
                .output_target
                .as_ref()
                .and_then(|target| target.bounds),
        );
        recording.snapshot.state = RecordingState::Stopping;
        if values.0 == RecordingOrigin::PushToTalk {
            recording.snapshot.hud = HudStateChanged {
                session_id: Some(values.1.clone()),
                state: HudState::Processing,
                message: Some("Transcribing...".to_string()),
            };
        }
        values
    };

    emit_recording_state(
        app,
        RecordingStateChanged {
            state: RecordingState::Stopping,
            transcription_id: None,
        },
    )
    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
    if origin == RecordingOrigin::PushToTalk {
        emit_hud_state(
            app,
            HudStateChanged {
                session_id: Some(session_id.clone()),
                state: HudState::Processing,
                message: Some("Transcribing...".to_string()),
            },
            target_bounds,
        )
        .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
    }
    emit_recording_state(
        app,
        RecordingStateChanged {
            state: RecordingState::Transcribing,
            transcription_id: None,
        },
    )
    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;

    let session = state
        .recording()
        .lock()
        .expect("recording mutex poisoned")
        .take_session()
        .ok_or_else(|| AppErrorDto::unknown("no active recording session"))?;

    match state.services().recording_pipeline().stop(&session) {
        Ok(result) => {
            if state
                .services()
                .settings()
                .map_err(|error| AppErrorDto::unknown(error.to_string()))?
                .history_enabled
            {
                state
                    .history()
                    .insert_completed(&result)
                    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
                app.emit("history_changed", ())
                    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
            }
            let dto = RecordingResultDto {
                session_id: result.session_id.clone(),
                origin: result.origin,
                raw_text: result.raw_text.clone(),
                deterministic_text: result.deterministic_text.clone(),
                final_text: result.final_text.clone(),
                stt_backend: result.stt_backend.clone(),
                cleanup_backend: result.cleanup_backend.clone(),
                stt_latency_ms: result.stt_latency_ms,
                cleanup_latency_ms: result.cleanup_latency_ms,
                cleanup_fallback_reason: result.cleanup_fallback_reason.clone(),
                peak_level: result.peak_level,
                status: result.status,
                output_method: result.output.method,
                output_result: result.output.result,
                output_message: result.output.message.clone(),
                application_name: result.active_window.application_name.clone(),
                window_title: result.active_window.window_title.clone(),
                duration_ms: result.duration_ms,
            };
            let hud_state = if result.output.method == OutputMethod::ClipboardCopyOnly {
                HudState::Clipboard
            } else {
                HudState::Inserted
            };

            {
                let mut recording = state.recording().lock().expect("recording mutex poisoned");
                recording.snapshot.state = RecordingState::Idle;
                if origin == RecordingOrigin::Scratch {
                    recording.snapshot.last_transcript = Some(result.final_text.clone());
                }
                recording.snapshot.last_error = None;
                if origin == RecordingOrigin::PushToTalk {
                    recording.snapshot.hud = HudStateChanged {
                        session_id: Some(session_id.clone()),
                        state: hud_state,
                        message: Some(result.output.message.clone()),
                    };
                }
            }

            emit_recording_state(
                app,
                RecordingStateChanged {
                    state: RecordingState::Idle,
                    transcription_id: Some(result.session_id.clone()),
                },
            )
            .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
            app.emit("transcription_completed", dto.clone())
                .map_err(|error| AppErrorDto::unknown(error.to_string()))?;

            if origin == RecordingOrigin::PushToTalk {
                emit_hud_state(
                    app,
                    HudStateChanged {
                        session_id: Some(session_id.clone()),
                        state: hud_state,
                        message: Some(result.output.message.clone()),
                    },
                    target_bounds,
                )
                .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
                schedule_hud_hide(
                    app.clone(),
                    session_id,
                    if hud_state == HudState::Inserted {
                        1_200
                    } else {
                        2_500
                    },
                );
            }
            Ok(dto)
        }
        Err(error) => {
            let app_error = map_pipeline_error(error);
            {
                let mut recording = state.recording().lock().expect("recording mutex poisoned");
                recording.snapshot.state = RecordingState::Error;
                recording.snapshot.last_error = Some(app_error.clone());
                if origin == RecordingOrigin::PushToTalk {
                    recording.snapshot.hud = HudStateChanged {
                        session_id: Some(session_id.clone()),
                        state: HudState::Error,
                        message: Some(app_error.message.clone()),
                    };
                }
            }
            emit_recording_state(
                app,
                RecordingStateChanged {
                    state: RecordingState::Error,
                    transcription_id: None,
                },
            )
            .map_err(|emit_error| AppErrorDto::unknown(emit_error.to_string()))?;
            if origin == RecordingOrigin::PushToTalk {
                emit_hud_state(
                    app,
                    HudStateChanged {
                        session_id: Some(session_id.clone()),
                        state: HudState::Error,
                        message: Some(app_error.message.clone()),
                    },
                    target_bounds,
                )
                .map_err(|emit_error| AppErrorDto::unknown(emit_error.to_string()))?;
                schedule_hud_hide(app.clone(), session_id, 2_500);
            }
            Err(app_error.into())
        }
    }
}

pub fn cancel_recording(
    app: &tauri::AppHandle,
    state: &ManagedAppState,
) -> Result<(), AppErrorDto> {
    let session = state
        .recording()
        .lock()
        .expect("recording mutex poisoned")
        .active_session
        .clone();
    if let Some(session) = session {
        state
            .services()
            .recording_pipeline()
            .cancel(&session)
            .map_err(map_pipeline_error)
            .map_err(AppErrorDto::from)?;
        {
            let mut recording = state.recording().lock().expect("recording mutex poisoned");
            recording.snapshot.state = RecordingState::Idle;
            recording.snapshot.hud = HudStateChanged {
                session_id: Some(session.capture.id.clone()),
                state: HudState::Hidden,
                message: None,
            };
            let _ = recording.take_session();
        }
        if session.origin == RecordingOrigin::PushToTalk {
            emit_hud_state(
                app,
                HudStateChanged {
                    session_id: Some(session.capture.id),
                    state: HudState::Hidden,
                    message: None,
                },
                None,
            )
            .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
        }
    }
    emit_recording_state(
        app,
        RecordingStateChanged {
            state: RecordingState::Idle,
            transcription_id: None,
        },
    )
    .map_err(|error| AppErrorDto::unknown(error.to_string()))
}

#[tauri::command]
pub fn recording_start_manual(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedAppState>,
) -> Result<(), AppErrorDto> {
    start_recording_with_origin(&app, &state, RecordingOrigin::Scratch)
}

#[tauri::command]
pub async fn recording_stop_manual(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedAppState>,
) -> Result<RecordingResultDto, AppErrorDto> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stop_recording(&app, &state))
        .await
        .map_err(|error| AppErrorDto::unknown(error.to_string()))?
}

#[tauri::command]
pub fn recording_cancel(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedAppState>,
) -> Result<(), AppErrorDto> {
    cancel_recording(&app, &state)
}

#[tauri::command]
pub fn recording_snapshot_get(
    state: tauri::State<'_, ManagedAppState>,
) -> banshee_core::domain::RecordingSnapshot {
    state
        .recording()
        .lock()
        .expect("recording mutex poisoned")
        .snapshot
        .clone()
}
