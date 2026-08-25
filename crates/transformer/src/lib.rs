//! Deterministic transcript cleanup for Banshee.

use anyhow::Result;
use banshee_contracts::domain::{CleanupEngine, CleanupOutput, CleanupRequest};
use std::time::Instant;

#[derive(Clone, Default)]
pub struct TranscriptCleanup;

impl CleanupEngine for TranscriptCleanup {
    fn cleanup(&self, request: CleanupRequest) -> Result<CleanupOutput> {
        let started = Instant::now();
        Ok(CleanupOutput {
            deterministic_text: deterministic_cleanup(&request),
            backend: "deterministic".into(),
            latency_ms: started.elapsed().as_millis() as u64,
        })
    }
}

fn deterministic_cleanup(request: &CleanupRequest) -> String {
    if request.profile.slug == "raw" {
        return request.raw_text.trim().to_string();
    }
    let mut text = request.raw_text.trim().to_string();
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
    let mut vocabulary = request.vocabulary.clone();
    vocabulary.sort_by_key(|entry| std::cmp::Reverse(entry.spoken_form.len()));
    for entry in vocabulary {
        text = replace_case_insensitive(&text, &entry.spoken_form, &entry.output_form, true);
    }
    text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if request.profile.slug == "commit" {
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
                .is_none_or(|c| !c.is_alphanumeric())
                && text[end..]
                    .chars()
                    .next()
                    .is_none_or(|c| !c.is_alphanumeric()));
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
    use banshee_contracts::domain::{DictionaryEntry, ProfileSummary};

    fn request(raw_text: &str) -> CleanupRequest {
        CleanupRequest {
            raw_text: raw_text.into(),
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
        }
    }

    #[test]
    fn preserves_case_and_applies_vocabulary() {
        let output = TranscriptCleanup
            .cleanup(request("Um banci hud works period"))
            .expect("cleanup");
        assert_eq!(output.deterministic_text, "Banshee HUD works.");
        assert_eq!(output.backend, "deterministic");
    }

    #[test]
    fn does_not_strip_like_inside_words() {
        let output = TranscriptCleanup
            .cleanup(request("I dislike regressions"))
            .expect("cleanup");
        assert_eq!(output.deterministic_text, "I dislike regressions.");
    }
}
