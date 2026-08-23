use anyhow::Result;
use banshee_core::domain::{ProfileStore, ProfileSummary};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};

struct BuiltInProfileSeed {
    id: &'static str,
    name: &'static str,
    slug: &'static str,
    description: &'static str,
    preserve_commands: bool,
    preserve_punctuation: bool,
    prefer_concise_output: bool,
    apply_repository_context: bool,
    enable_cleanup_llm: bool,
    file_reference_style: &'static str,
}

#[derive(Clone)]
pub struct SqliteProfileRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteProfileRepository {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }

    pub fn seed_builtin_profiles(&self) -> Result<()> {
        let profiles = [
            BuiltInProfileSeed {
                id: "profile-raw",
                name: "Raw",
                slug: "raw",
                description: "Minimal cleanup for near-verbatim transcription output.",
                preserve_commands: true,
                preserve_punctuation: true,
                prefer_concise_output: false,
                apply_repository_context: false,
                enable_cleanup_llm: false,
                file_reference_style: "none",
            },
            BuiltInProfileSeed {
                id: "profile-agent",
                name: "Agent",
                slug: "agent",
                description: "Balanced instructions for coding agents and tool-driven workflows.",
                preserve_commands: true,
                preserve_punctuation: true,
                prefer_concise_output: false,
                apply_repository_context: true,
                enable_cleanup_llm: false,
                file_reference_style: "agent_at_path",
            },
            BuiltInProfileSeed {
                id: "profile-codex",
                name: "Codex",
                slug: "codex",
                description: "Structured output tuned for direct agent execution prompts.",
                preserve_commands: true,
                preserve_punctuation: true,
                prefer_concise_output: true,
                apply_repository_context: true,
                enable_cleanup_llm: false,
                file_reference_style: "agent_at_path",
            },
            BuiltInProfileSeed {
                id: "profile-claude-code",
                name: "Claude Code",
                slug: "claude-code",
                description: "Developer-oriented cleanup for coding conversations and patch requests.",
                preserve_commands: true,
                preserve_punctuation: true,
                prefer_concise_output: false,
                apply_repository_context: true,
                enable_cleanup_llm: false,
                file_reference_style: "agent_at_path",
            },
            BuiltInProfileSeed {
                id: "profile-terminal",
                name: "Terminal",
                slug: "terminal",
                description: "Concise output tuned for shell and CLI workflows.",
                preserve_commands: true,
                preserve_punctuation: true,
                prefer_concise_output: true,
                apply_repository_context: false,
                enable_cleanup_llm: false,
                file_reference_style: "plain_path",
            },
            BuiltInProfileSeed {
                id: "profile-commit",
                name: "Commit",
                slug: "commit",
                description: "Short, high-signal output shaped for commit summaries.",
                preserve_commands: false,
                preserve_punctuation: false,
                prefer_concise_output: true,
                apply_repository_context: true,
                enable_cleanup_llm: false,
                file_reference_style: "plain_path",
            },
            BuiltInProfileSeed {
                id: "profile-documentation",
                name: "Documentation",
                slug: "documentation",
                description: "Readable prose output for docs, notes, and explanations.",
                preserve_commands: false,
                preserve_punctuation: true,
                prefer_concise_output: false,
                apply_repository_context: true,
                enable_cleanup_llm: false,
                file_reference_style: "plain_path",
            },
        ];

        let connection = self.connection.lock().expect("profile mutex poisoned");
        for profile in profiles {
            connection.execute(
                "INSERT OR IGNORE INTO profiles (
                    id, name, slug, built_in, description,
                    live_partial_transcript, remove_fillers, resolve_corrections,
                    apply_dictionary, apply_repository_context, enable_cleanup_llm,
                    preserve_commands, preserve_punctuation, prefer_concise_output,
                    file_reference_style, trailing_whitespace_policy,
                    cleanup_prompt_template, created_at, updated_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5,
                    0, 1, 1,
                    1, ?6, ?7,
                    ?8, ?9, ?10,
                    ?11, 'trim',
                    NULL, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
                )",
                params![
                    profile.id,
                    profile.name,
                    profile.slug,
                    true,
                    profile.description,
                    profile.apply_repository_context,
                    profile.enable_cleanup_llm,
                    profile.preserve_commands,
                    profile.preserve_punctuation,
                    profile.prefer_concise_output,
                    profile.file_reference_style,
                ],
            )?;
        }

        Ok(())
    }
}

impl ProfileStore for SqliteProfileRepository {
    fn list(&self) -> Result<Vec<ProfileSummary>> {
        let connection = self.connection.lock().expect("profile mutex poisoned");
        let mut statement = connection.prepare(
            "SELECT id, name, slug, description, built_in
             FROM profiles
             ORDER BY built_in DESC, name ASC",
        )?;

        let rows = statement.query_map([], |row| {
            Ok(ProfileSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                slug: row.get(2)?,
                description: row.get(3)?,
                built_in: row.get(4)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn default_profile(&self) -> Result<ProfileSummary> {
        let connection = self.connection.lock().expect("profile mutex poisoned");
        connection
            .query_row(
                "SELECT id, name, slug, description, built_in
                 FROM profiles
                 WHERE slug = 'agent'
                 LIMIT 1",
                [],
                |row| {
                    Ok(ProfileSummary {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        slug: row.get(2)?,
                        description: row.get(3)?,
                        built_in: row.get(4)?,
                    })
                },
            )
            .map_err(Into::into)
    }
}
