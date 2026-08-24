use anyhow::{Result, bail};
use banshee_core::domain::{DictionaryEntry, DictionaryStore};
use rusqlite::{Connection, params};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SqliteDictionaryRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteDictionaryRepository {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }

    fn normalize(entries: Vec<DictionaryEntry>) -> Result<Vec<DictionaryEntry>> {
        if entries.len() > 200 {
            bail!("custom vocabulary is limited to 200 entries");
        }
        let mut seen = HashSet::new();
        let mut normalized = Vec::new();
        for entry in entries {
            let spoken_form = entry.spoken_form.trim().to_string();
            let output_form = entry.output_form.trim().to_string();
            if spoken_form.is_empty() || output_form.is_empty() {
                bail!("vocabulary terms cannot be empty");
            }
            if spoken_form.chars().count() > 120 || output_form.chars().count() > 120 {
                bail!("vocabulary terms must be 120 characters or fewer");
            }
            if seen.insert(spoken_form.to_lowercase()) {
                normalized.push(DictionaryEntry {
                    spoken_form,
                    output_form,
                });
            }
        }
        Ok(normalized)
    }
}

impl DictionaryStore for SqliteDictionaryRepository {
    fn list_global(&self) -> Result<Vec<DictionaryEntry>> {
        let connection = self.connection.lock().expect("dictionary mutex poisoned");
        let mut statement = connection.prepare(
            "SELECT spoken_form, output_form
             FROM dictionary_entries
             WHERE scope = 'global' AND enabled = 1
             ORDER BY priority DESC, created_at ASC, id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(DictionaryEntry {
                spoken_form: row.get(0)?,
                output_form: row.get(1)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn replace_global(&self, entries: Vec<DictionaryEntry>) -> Result<Vec<DictionaryEntry>> {
        let entries = Self::normalize(entries)?;
        let mut connection = self.connection.lock().expect("dictionary mutex poisoned");
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM dictionary_entries WHERE scope = 'global'", [])?;
        for (index, entry) in entries.iter().enumerate() {
            transaction.execute(
                "INSERT INTO dictionary_entries (
                    id, scope, project_id, spoken_form, output_form, enabled, priority, match_mode
                 ) VALUES (?1, 'global', NULL, ?2, ?3, 1, 0, 'exact_phrase')",
                params![
                    format!("dictionary-global-{index}"),
                    entry.spoken_form,
                    entry.output_form
                ],
            )?;
        }
        transaction.commit()?;
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_and_deduplicates_global_entries() {
        let mut connection = Connection::open_in_memory().expect("database");
        crate::migrate(&mut connection).expect("migrations");
        let repository = SqliteDictionaryRepository::new(Arc::new(Mutex::new(connection)));
        let entries = repository
            .replace_global(vec![
                DictionaryEntry {
                    spoken_form: "banci".into(),
                    output_form: "Banshee".into(),
                },
                DictionaryEntry {
                    spoken_form: "BANCI".into(),
                    output_form: "ignored".into(),
                },
            ])
            .expect("replace");
        assert_eq!(entries.len(), 1);
        assert_eq!(
            repository.list_global().expect("list")[0].output_form,
            "Banshee"
        );
    }
}
