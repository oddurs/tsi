use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::Engine;

/// Embedded engine database (compiled into the binary).
const EMBEDDED_ENGINES: &str = include_str!("../../data/engines.toml");

/// Container for TOML deserialization.
#[derive(Deserialize)]
struct EngineFile {
    engine: Vec<Engine>,
}

/// Database of available rocket engines.
#[derive(Debug, Clone)]
pub struct EngineDatabase {
    engines: Vec<Engine>,
}

impl EngineDatabase {
    /// Load the embedded engine database.
    pub fn load_embedded() -> Result<Self> {
        let file: EngineFile =
            toml::from_str(EMBEDDED_ENGINES).context("Failed to parse embedded engine database")?;
        Ok(Self {
            engines: file.engine,
        })
    }

    /// Load an engine database from a TOML file.
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read engine file: {}", path.display()))?;
        let file: EngineFile = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse engine file: {}", path.display()))?;
        Ok(Self {
            engines: file.engine,
        })
    }

    /// Get an engine by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&Engine> {
        let name_lower = name.to_lowercase();
        self.engines
            .iter()
            .find(|e| e.name.to_lowercase() == name_lower)
    }

    /// List all available engines.
    pub fn list(&self) -> &[Engine] {
        &self.engines
    }

    /// Get available engine names.
    pub fn names(&self) -> Vec<&str> {
        self.engines.iter().map(|e| e.name.as_str()).collect()
    }

    /// Suggest similar engine names for a typo.
    /// Returns up to 3 suggestions sorted by similarity.
    pub fn suggest(&self, query: &str) -> Vec<&str> {
        let query_lower = query.to_lowercase();
        let mut scored: Vec<_> = self
            .engines
            .iter()
            .map(|e| {
                let name_lower = e.name.to_lowercase();

                // Strong preference for substring/prefix matches
                let score = if name_lower.starts_with(&query_lower) {
                    // Prefix match - best
                    0
                } else if name_lower.contains(&query_lower) {
                    // Substring match - very good
                    1
                } else if query_lower.starts_with(&name_lower) {
                    // Query is longer prefix
                    2
                } else {
                    // Fall back to edit distance
                    edit_distance(&query_lower, &name_lower) + 3
                };

                (e.name.as_str(), score)
            })
            .collect();

        // Sort by score (lower is better)
        scored.sort_by_key(|(_, score)| *score);

        // Return top suggestions with reasonable similarity
        scored
            .into_iter()
            .filter(|(_, score)| *score <= 6) // Reasonable threshold
            .take(3)
            .map(|(name, _)| name)
            .collect()
    }
}

/// Calculate edit distance (Levenshtein) between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1) // deletion
                .min(curr[j - 1] + 1) // insertion
                .min(prev[j - 1] + cost); // substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

impl Default for EngineDatabase {
    fn default() -> Self {
        Self::load_embedded().expect("Embedded engine database should be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_embedded_database() {
        let db = EngineDatabase::load_embedded().unwrap();
        assert!(!db.engines.is_empty());
    }

    #[test]
    fn get_engine_by_name() {
        let db = EngineDatabase::default();

        let merlin = db.get("Merlin-1D").unwrap();
        assert_eq!(merlin.name, "Merlin-1D");

        let raptor = db.get("raptor-2").unwrap(); // case-insensitive
        assert_eq!(raptor.name, "Raptor-2");
    }

    #[test]
    fn get_unknown_engine() {
        let db = EngineDatabase::default();
        assert!(db.get("NotARealEngine").is_none());
    }

    #[test]
    fn list_engines() {
        let db = EngineDatabase::default();
        let engines = db.list();
        assert!(engines.len() >= 10);

        let names = db.names();
        assert!(names.contains(&"Merlin-1D"));
        assert!(names.contains(&"Raptor-2"));
        assert!(names.contains(&"RS-25"));
    }

    #[test]
    fn suggest_similar_names() {
        let db = EngineDatabase::default();

        // Prefix match
        let suggestions = db.suggest("raptor");
        assert!(suggestions.contains(&"Raptor-2"));
        assert!(suggestions.contains(&"Raptor-Vacuum"));

        // Prefix match
        let suggestions = db.suggest("merlin");
        assert!(suggestions.contains(&"Merlin-1D"));
        assert!(suggestions.contains(&"Merlin-Vacuum"));

        // Close typo
        let suggestions = db.suggest("rapter-2");
        assert!(suggestions.contains(&"Raptor-2"));
    }
}
