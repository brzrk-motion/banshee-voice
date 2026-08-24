use crate::{
    app_state::{
        ManagedAppState, RecordingTrigger,
        ipc::{AppErrorDto, RecordingResultDto},
    },
    events::hud::{emit_hud_state, emit_recording_state},
};
use banshee_audio::AudioError;
use banshee_core::{
    domain::{
        AppError, AppErrorCode, FallbackUsed, HudState, HudStateChanged, RecordingState,
        RecordingStateChanged,
    },
    pipeline::PipelineError,
};
use banshee_injector::InjectorError;
use banshee_stt::SttError;
use tauri::Emitter;

fn map_pipeline_error(error: anyhow::Error) -> AppError {
    if let Some(audio_error) = error.downcast_ref::<AudioError>() {
        return AppError {
            code: match audio_error {
                AudioError::MicrophoneUnavailable => AppErrorCode::MicrophoneUnavailable,
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

pub fn start_recording_with_trigger(
    app: &tauri::AppHandle,
    state: &ManagedAppState,
    trigger: RecordingTrigger,
) -> Result<(), AppErrorDto> {
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

    let session = state
        .services()
        .recording_pipeline()
        .start_manual()
        .map_err(map_pipeline_error)
        .map_err(AppErrorDto::from)?;

    {
        let mut recording = state.recording().lock().expect("recording mutex poisoned");
        recording
            .begin_session(session, trigger)
            .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
        recording.snapshot.state = RecordingState::Recording;
        recording.snapshot.hud = HudStateChanged {
            state: HudState::Listening,
            message: Some("Listening for dictation".to_string()),
            level: Some(0.0),
            live_transcript: None,
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
    emit_hud_state(
        app,
        HudStateChanged {
            state: HudState::Listening,
            message: Some("Listening for dictation".to_string()),
            level: Some(0.0),
            live_transcript: None,
        },
    )
    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;

    Ok(())
}

pub fn stop_recording(
    app: &tauri::AppHandle,
    state: &ManagedAppState,
) -> Result<RecordingResultDto, AppErrorDto> {
    {
        let mut recording = state.recording().lock().expect("recording mutex poisoned");
        let Some(session) = recording.active_session.clone() else {
            return Err(AppErrorDto::unknown("no active recording session"));
        };
        recording.snapshot.state = RecordingState::Stopping;
        let _ = session;
    }

    emit_recording_state(
        app,
        RecordingStateChanged {
            state: RecordingState::Stopping,
            transcription_id: None,
        },
    )
    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
    emit_hud_state(
        app,
        HudStateChanged {
            state: HudState::Processing,
            message: Some("Processing local transcript".to_string()),
            level: None,
            live_transcript: None,
        },
    )
    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
    emit_recording_state(
        app,
        RecordingStateChanged {
            state: RecordingState::Transcribing,
            transcription_id: None,
        },
    )
    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;

    let (session, trigger) = {
        let mut recording = state.recording().lock().expect("recording mutex poisoned");
        recording.take_session()
    }
    .ok_or_else(|| AppErrorDto::unknown("no active recording session"))?;

    match state
        .services()
        .recording_pipeline()
        .stop(&session, trigger != RecordingTrigger::Manual)
    {
        Ok(result) => {
            let history_enabled = state
                .services()
                .settings()
                .map_err(|error| AppErrorDto::unknown(error.to_string()))?
                .history_enabled;
            if history_enabled {
                state
                    .history()
                    .insert_completed(&result)
                    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
                app.emit("history_changed", ())
                    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
            }
            let dto = RecordingResultDto {
                session_id: result.session_id.clone(),
                raw_text: result.raw_text.clone(),
                deterministic_text: result.deterministic_text.clone(),
                final_text: result.final_text.clone(),
                stt_backend: result.stt_backend.clone(),
                peak_level: result.peak_level,
                status: result.status,
                output_method: result.output.method,
                output_result: result.output.result,
                output_message: result.output.message.clone(),
                application_name: result.active_window.application_name.clone(),
                window_title: result.active_window.window_title.clone(),
                duration_ms: result.duration_ms,
            };

            {
                let mut recording = state.recording().lock().expect("recording mutex poisoned");
                recording.snapshot.state = RecordingState::Idle;
                recording.snapshot.last_transcript = Some(result.final_text.clone());
                recording.snapshot.last_error = None;
                recording.snapshot.hud = HudStateChanged {
                    state: HudState::Complete,
                    message: Some(result.output.message.clone()),
                    level: Some(result.peak_level),
                    live_transcript: Some(result.final_text.clone()),
                };
            }

            emit_recording_state(
                app,
                RecordingStateChanged {
                    state: if trigger == RecordingTrigger::Manual {
                        RecordingState::Idle
                    } else {
                        RecordingState::Inserting
                    },
                    transcription_id: Some(result.session_id.clone()),
                },
            )
            .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
            emit_recording_state(
                app,
                RecordingStateChanged {
                    state: RecordingState::Idle,
                    transcription_id: Some(result.session_id.clone()),
                },
            )
            .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
            emit_hud_state(
                app,
                HudStateChanged {
                    state: HudState::Complete,
                    message: Some(result.output.message),
                    level: Some(result.peak_level),
                    live_transcript: Some(result.final_text),
                },
            )
            .map_err(|error| AppErrorDto::unknown(error.to_string()))?;

            Ok(dto)
        }
        Err(error) => {
            let app_error = map_pipeline_error(error);
            {
                let mut recording = state.recording().lock().expect("recording mutex poisoned");
                recording.snapshot.state = RecordingState::Error;
                recording.snapshot.last_error = Some(app_error.clone());
                recording.snapshot.hud = HudStateChanged {
                    state: HudState::Error,
                    message: Some(app_error.message.clone()),
                    level: None,
                    live_transcript: None,
                };
            }
            emit_recording_state(
                app,
                RecordingStateChanged {
                    state: RecordingState::Error,
                    transcription_id: None,
                },
            )
            .map_err(|emit_error| AppErrorDto::unknown(emit_error.to_string()))?;
            emit_hud_state(
                app,
                HudStateChanged {
                    state: HudState::Error,
                    message: Some(app_error.message.clone()),
                    level: None,
                    live_transcript: None,
                },
            )
            .map_err(|emit_error| AppErrorDto::unknown(emit_error.to_string()))?;
            Err(app_error.into())
        }
    }
}

pub fn cancel_recording(
    app: &tauri::AppHandle,
    state: &ManagedAppState,
) -> Result<(), AppErrorDto> {
    let active_session = {
        let recording = state.recording().lock().expect("recording mutex poisoned");
        recording.active_session.clone()
    };

    if let Some(session) = active_session {
        state
            .services()
            .recording_pipeline()
            .cancel(&session)
            .map_err(map_pipeline_error)
            .map_err(AppErrorDto::from)?;

        let mut recording = state.recording().lock().expect("recording mutex poisoned");
        recording.snapshot.state = RecordingState::Idle;
        recording.snapshot.hud = HudStateChanged {
            state: HudState::Hidden,
            message: None,
            level: None,
            live_transcript: None,
        };
        let _ = recording.take_session();
    }

    emit_recording_state(
        app,
        RecordingStateChanged {
            state: RecordingState::Idle,
            transcription_id: None,
        },
    )
    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;
    emit_hud_state(
        app,
        HudStateChanged {
            state: HudState::Hidden,
            message: None,
            level: None,
            live_transcript: None,
        },
    )
    .map_err(|error| AppErrorDto::unknown(error.to_string()))?;

    Ok(())
}

#[tauri::command]
pub fn recording_start_manual(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedAppState>,
) -> Result<(), AppErrorDto> {
    start_recording_with_trigger(&app, &state, RecordingTrigger::Manual)
}

#[tauri::command]
pub fn recording_stop_manual(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedAppState>,
) -> Result<RecordingResultDto, AppErrorDto> {
    stop_recording(&app, &state)
}

#[tauri::command]
pub fn recording_cancel(
    app: tauri::AppHandle,
    state: tauri::State<'_, ManagedAppState>,
) -> Result<(), AppErrorDto> {
    cancel_recording(&app, &state)
}
