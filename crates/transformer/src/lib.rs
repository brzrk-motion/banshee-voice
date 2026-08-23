//! Transcript transformation and cleanup logic for Banshee.

use anyhow::Result;
use banshee_core::domain::{CleanupEngine, CleanupOutput, CleanupRequest};

#[derive(Debug, Default, Clone, Copy)]
pub struct DeterministicCleanup;

impl CleanupEngine for DeterministicCleanup {
    fn cleanup(&self, request: CleanupRequest) -> Result<CleanupOutput> {
        let mut text = request.raw_text.to_lowercase();

        for correction_marker in ["scratch that", "correction"] {
            if let Some((_, corrected)) = text.rsplit_once(correction_marker) {
                text = corrected.trim().to_string();
            }
        }

        for filler in ["um ", "uh ", "like "] {
            while text.contains(filler) {
                text = text.replacen(filler, "", 1);
            }
        }

        for (spoken, punctuation) in [
            (" comma", ","),
            (" period", "."),
            (" question mark", "?"),
            (" exclamation mark", "!"),
        ] {
            text = text.replace(spoken, punctuation);
        }

        let mut deterministic_text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if request.profile.slug == "raw" {
            deterministic_text = request.raw_text.trim().to_string();
        }

        let final_text = if request.profile.slug == "commit" {
            deterministic_text.trim_end_matches('.').to_string()
        } else if deterministic_text.ends_with(['.', '!', '?']) {
            deterministic_text.clone()
        } else {
            format!("{deterministic_text}.")
        };

        Ok(CleanupOutput {
            deterministic_text,
            final_text,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use banshee_core::domain::ProfileSummary;

    fn profile(slug: &str) -> ProfileSummary {
        ProfileSummary {
            id: format!("profile-{slug}"),
            name: slug.to_string(),
            slug: slug.to_string(),
            description: String::new(),
            built_in: true,
        }
    }

    #[test]
    fn strips_fillers_and_punctuation_words() {
        let cleanup = DeterministicCleanup;
        let output = cleanup
            .cleanup(CleanupRequest {
                raw_text: "um update the file period".to_string(),
                profile: profile("agent"),
            })
            .expect("cleanup should succeed");

        assert_eq!(output.deterministic_text, "update the file.");
        assert_eq!(output.final_text, "update the file.");
    }

    #[test]
    fn keeps_commit_output_without_trailing_period() {
        let cleanup = DeterministicCleanup;
        let output = cleanup
            .cleanup(CleanupRequest {
                raw_text: "ship the fix period".to_string(),
                profile: profile("commit"),
            })
            .expect("cleanup should succeed");

        assert_eq!(output.final_text, "ship the fix");
    }
}
