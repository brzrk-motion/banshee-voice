use anyhow::{Result, anyhow};
use banshee_core::domain::{
    AccelerationPreference, HistoryItem, HistoryPage, OutputMethod, OutputResultKind,
    PipelineRunResult, PipelineRunStatus,
};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SqliteTranscriptionRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteTranscriptionRepository {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }

    pub fn insert_completed(&self, result: &PipelineRunResult) -> Result<()> {
        let word_count = result.final_text.split_whitespace().count() as i64;
        let character_count = result.final_text.chars().count() as i64;
        let connection = self
            .connection
            .lock()
            .expect("transcription mutex poisoned");

        connection.execute(
            "INSERT INTO transcriptions (
                id, created_at, updated_at, status, duration_ms,
                audio_retained, audio_path, raw_text, deterministic_text, final_text,
                word_count, character_count, source_application, window_title, session_type,
                project_id, profile_id, speech_model_id, cleanup_model_id, stt_backend,
                cleanup_backend, acceleration_requested, acceleration_actual,
                stt_latency_ms, cleanup_latency_ms, total_latency_ms,
                output_method, output_result, error_code, error_message
             ) VALUES (
                ?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?2, ?3, 0, NULL, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                NULL, ?12, NULL, NULL, ?13, 'deterministic', ?14, ?14,
                NULL, NULL, NULL, ?15, ?16, NULL, NULL
             )",
            params![
                result.session_id,
                encode_status(result.status),
                result.duration_ms as i64,
                result.raw_text,
                result.deterministic_text,
                result.final_text,
                word_count,
                character_count,
                result.active_window.application_name,
                result.active_window.window_title,
                result.session_type.as_str(),
                result.profile_id,
                result.stt_backend,
                encode_acceleration(result.acceleration_preference),
                encode_output_method(result.output.method),
                encode_output_result(result.output.result),
            ],
        )?;

        Ok(())
    }

    pub fn list(&self, limit: usize, cursor: Option<&str>) -> Result<HistoryPage> {
        let limit = limit.clamp(1, 100);
        let fetch_limit = limit + 1;
        let connection = self
            .connection
            .lock()
            .expect("transcription mutex poisoned");
        let mut items = Vec::with_capacity(fetch_limit);

        if let Some(cursor) = cursor {
            let (created_at, id) = cursor
                .split_once('|')
                .ok_or_else(|| anyhow!("invalid history cursor"))?;
            let mut statement = connection.prepare(
                "SELECT id, created_at, final_text
                 FROM transcriptions
                 WHERE final_text IS NOT NULL
                   AND (created_at < ?1 OR (created_at = ?1 AND id < ?2))
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?3",
            )?;
            let rows = statement.query_map(params![created_at, id, fetch_limit as i64], map_row)?;
            for row in rows {
                items.push(row?);
            }
        } else {
            let mut statement = connection.prepare(
                "SELECT id, created_at, final_text
                 FROM transcriptions
                 WHERE final_text IS NOT NULL
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?1",
            )?;
            let rows = statement.query_map([fetch_limit as i64], map_row)?;
            for row in rows {
                items.push(row?);
            }
        }

        let has_more = items.len() > limit;
        items.truncate(limit);
        let next_cursor = if has_more {
            items
                .last()
                .map(|item| format!("{}|{}", item.created_at, item.id))
        } else {
            None
        };

        Ok(HistoryPage { items, next_cursor })
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryItem> {
    Ok(HistoryItem {
        id: row.get(0)?,
        created_at: row.get(1)?,
        final_text: row.get(2)?,
    })
}

fn encode_status(value: PipelineRunStatus) -> &'static str {
    match value {
        PipelineRunStatus::Completed => "completed",
        PipelineRunStatus::FallbackUsed => "fallback_used",
    }
}

fn encode_acceleration(value: AccelerationPreference) -> &'static str {
    match value {
        AccelerationPreference::Auto => "auto",
        AccelerationPreference::Cpu => "cpu",
        AccelerationPreference::Gpu => "gpu",
    }
}

fn encode_output_method(value: OutputMethod) -> &'static str {
    match value {
        OutputMethod::DirectInsert => "direct_insert",
        OutputMethod::ClipboardPaste => "clipboard_paste",
        OutputMethod::ClipboardCopyOnly => "clipboard_copy_only",
        OutputMethod::None => "none",
    }
}

fn encode_output_result(value: OutputResultKind) -> &'static str {
    match value {
        OutputResultKind::Success => "success",
        OutputResultKind::Fallback => "fallback",
        OutputResultKind::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use banshee_core::domain::{ActiveWindowInfo, OutputResponse, RecordingOrigin, SessionType};

    fn repository() -> SqliteTranscriptionRepository {
        let mut connection = Connection::open_in_memory().expect("database should open");
        crate::migrate(&mut connection).expect("migration should apply");
        connection.execute(
            "INSERT INTO profiles (id, name, slug, built_in, description) VALUES ('profile-agent', 'Agent', 'agent', 1, '')",
            [],
        ).expect("profile should insert");
        SqliteTranscriptionRepository::new(Arc::new(Mutex::new(connection)))
    }

    fn result(id: &str, text: &str) -> PipelineRunResult {
        PipelineRunResult {
            session_id: id.to_string(),
            origin: RecordingOrigin::Scratch,
            raw_text: text.to_string(),
            deterministic_text: text.to_string(),
            final_text: text.to_string(),
            stt_backend: "test".to_string(),
            peak_level: 0.5,
            status: PipelineRunStatus::Completed,
            output: OutputResponse {
                method: OutputMethod::None,
                result: OutputResultKind::Success,
                message: "ready".to_string(),
            },
            active_window: ActiveWindowInfo {
                application_name: "Banshee".to_string(),
                window_title: "Transcribe".to_string(),
            },
            duration_ms: 500,
            profile_id: "profile-agent".to_string(),
            acceleration_preference: AccelerationPreference::Auto,
            session_type: SessionType::Windows,
        }
    }

    #[test]
    fn stores_and_lists_text_without_audio() {
        let repository = repository();
        repository
            .insert_completed(&result("session-1", "hello world"))
            .expect("insert should work");

        let page = repository.list(30, None).expect("list should work");
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].final_text, "hello world");

        let connection = repository.connection.lock().expect("database lock");
        let (retained, path): (i64, Option<String>) = connection
            .query_row(
                "SELECT audio_retained, audio_path FROM transcriptions WHERE id = 'session-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("row should exist");
        assert_eq!(retained, 0);
        assert!(path.is_none());
    }

    #[test]
    fn paginates_in_stable_newest_first_order() {
        let repository = repository();
        repository
            .insert_completed(&result("session-a", "first"))
            .expect("insert should work");
        repository
            .insert_completed(&result("session-b", "second"))
            .expect("insert should work");

        let first = repository.list(1, None).expect("first page should load");
        assert_eq!(first.items[0].id, "session-b");
        let second = repository
            .list(1, first.next_cursor.as_deref())
            .expect("second page should load");
        assert_eq!(second.items[0].id, "session-a");
    }
}
