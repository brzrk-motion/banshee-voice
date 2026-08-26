//! Built-in deterministic transcript cleanup plugin.

use anyhow::Result;
use banshee_contracts::domain::{
    PluginExecutionContext, PluginExecutionOutput, PluginManifest, PluginRuntimeState,
    PluginRuntimeStatus, TextTransformPlugin,
};
use std::collections::BTreeMap;

pub const TRANSCRIPT_CLEANUP_ID: &str = "banshee.transcript-cleanup";

#[derive(Clone, Default)]
pub struct TranscriptCleanup;

impl TextTransformPlugin for TranscriptCleanup {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: TRANSCRIPT_CLEANUP_ID.into(),
            name: "Transcript Cleanup".into(),
            description: "Removes filler words, applies spoken punctuation and dictionary terms, and normalizes the transcript before other plugins run.".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            author: "Banshee".into(),
            stage: "First after transcription".into(),
            settings: vec![],
        }
    }

    fn runtime_status(&self) -> PluginRuntimeStatus {
        PluginRuntimeStatus {
            state: PluginRuntimeState::Ready,
            downloaded_bytes: 0,
            total_bytes: None,
            message: None,
        }
    }

    fn transform(
        &self,
        context: &PluginExecutionContext,
        _settings: &BTreeMap<String, String>,
    ) -> Result<PluginExecutionOutput> {
        Ok(PluginExecutionOutput {
            text: deterministic_cleanup(context),
            backend: "deterministic".into(),
        })
    }
}

fn deterministic_cleanup(context: &PluginExecutionContext) -> String {
    if context.profile.slug == "raw" {
        return context.current_text.trim().to_string();
    }
    let mut text = context.current_text.trim().to_string();
    let lowered = text.to_ascii_lowercase();
    for marker in ["scratch that", "correction"] {
        if let Some(index) = lowered.rfind(marker) {
            text = text[index + marker.len()..].trim().to_string();
            break;
        }
    }
    text = text
        .split_whitespace()
        .filter(|token| {
            let normalized = token.trim_matches(|c: char| !c.is_alphanumeric());
            !matches!(normalized.to_lowercase().as_str(), "um" | "uh")
                && !token.eq_ignore_ascii_case("like,")
        })
        .collect::<Vec<_>>()
        .join(" ");
    for (spoken, punctuation) in [
        (" comma", ","),
        (" period", "."),
        (" question mark", "?"),
        (" exclamation mark", "!"),
    ] {
        text = replace_case_insensitive(&text, spoken, punctuation, false);
    }
    let mut vocabulary = context.vocabulary.clone();
    vocabulary.sort_by_key(|entry| std::cmp::Reverse(entry.spoken_form.len()));
    for entry in vocabulary {
        text = replace_case_insensitive(&text, &entry.spoken_form, &entry.output_form, true);
    }
    text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if context.profile.slug == "commit" {
        text.trim_end_matches('.').to_string()
    } else if text.ends_with(['.', '!', '?']) || text.is_empty() {
        text
    } else {
        format!("{text}.")
    }
}

fn replace_case_insensitive(
    text: &str,
    needle: &str,
    replacement: &str,
    word_boundaries: bool,
) -> String {
    if needle.is_empty() {
        return text.to_string();
    }
    let lowered = text.to_ascii_lowercase();
    let needle_lower = needle.to_ascii_lowercase();
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(relative) = lowered[cursor..].find(&needle_lower) {
        let start = cursor + relative;
        let end = start + needle_lower.len();
        let boundary_ok = !word_boundaries
            || (text[..start]
                .chars()
                .next_back()
                .is_none_or(|character| !character.is_alphanumeric())
                && text[end..]
                    .chars()
                    .next()
                    .is_none_or(|character| !character.is_alphanumeric()));
        if boundary_ok {
            result.push_str(&text[cursor..start]);
            result.push_str(replacement);
            cursor = end;
        } else {
            result.push_str(&text[cursor..end]);
            cursor = end;
        }
    }
    result.push_str(&text[cursor..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use banshee_contracts::domain::{
        DictionaryEntry, ProfileSummary, RecordingOrigin, TextTransformPlugin,
    };

    fn context(current_text: &str) -> PluginExecutionContext {
        PluginExecutionContext {
            raw_text: current_text.into(),
            cleaned_text: current_text.into(),
            current_text: current_text.into(),
            profile: ProfileSummary {
                id: "profile-agent".into(),
                name: "Agent".into(),
                slug: "agent".into(),
                description: String::new(),
                built_in: true,
            },
            vocabulary: vec![DictionaryEntry {
                spoken_form: "banci hud".into(),
                output_form: "Banshee HUD".into(),
            }],
            active_application: "Editor".into(),
            recording_origin: RecordingOrigin::Scratch,
        }
    }

    #[test]
    fn is_an_always_ready_plugin_without_settings() {
        let plugin = TranscriptCleanup;
        assert_eq!(plugin.manifest().id, TRANSCRIPT_CLEANUP_ID);
        assert!(plugin.manifest().settings.is_empty());
        assert_eq!(plugin.runtime_status().state, PluginRuntimeState::Ready);
    }

    #[test]
    fn preserves_case_and_applies_vocabulary() {
        let output = TranscriptCleanup
            .transform(&context("Um banci hud works period"), &BTreeMap::new())
            .expect("cleanup");
        assert_eq!(output.text, "Banshee HUD works.");
        assert_eq!(output.backend, "deterministic");
    }

    #[test]
    fn does_not_strip_like_inside_words() {
        let output = TranscriptCleanup
            .transform(&context("I dislike regressions"), &BTreeMap::new())
            .expect("cleanup");
        assert_eq!(output.text, "I dislike regressions.");
    }
}
