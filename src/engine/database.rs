use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;

use super::Engine;

/// Embedded engine database (compiled into the binary).
const EMBEDDED_ENGINES: &str = include_str!("../../data/engines.toml");

/// Container for TOML deserialization.
#[derive(Deserialize)]
struct EngineFile {
    engine: Vec<Engine>,
}

/// Why an engine database couldn't be loaded.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DatabaseError {
    /// The file couldn't be read.
    #[error("couldn't read engine file {}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    /// The TOML is malformed, or an engine in it is physically impossible
    /// (every engine is validated by [`Engine::new`] as it loads).
    #[error("invalid engine data in {origin}: {message}")]
    Invalid {
        /// The file, or "the built-in database"
        origin: String,
        /// What was wrong, with the line and column where TOML gives them
        message: String,
    },
}

/// Database of available rocket engines.
#[derive(Debug, Clone)]
pub struct EngineDatabase {
    engines: Vec<Engine>,
}

impl EngineDatabase {
    /// The engine database compiled into tsi, parsed once and shared.
    ///
    /// The embedded data is checked by the crate's own tests, so this can't
    /// fail at run time.
    pub fn builtin() -> &'static EngineDatabase {
        static BUILTIN: OnceLock<EngineDatabase> = OnceLock::new();
        BUILTIN.get_or_init(|| {
            #[expect(
                clippy::expect_used,
                reason = "the embedded database is validated by tests; failure is a build defect"
            )]
            Self::parse(EMBEDDED_ENGINES, "the built-in database")
                .expect("the built-in engine database is valid")
        })
    }

    /// Load the embedded engine database.
    ///
    /// Returns a copy of [`EngineDatabase::builtin`]; prefer that when a
    /// reference is enough.
    pub fn load_embedded() -> Result<Self, DatabaseError> {
        Self::parse(EMBEDDED_ENGINES, "the built-in database")
    }

    /// Load an engine database from a TOML file.
    ///
    /// # Errors
    ///
    /// [`DatabaseError::Io`] if the file can't be read, or
    /// [`DatabaseError::Invalid`] if its TOML is malformed or any engine is
    /// physically impossible.
    pub fn load_from_file(path: &Path) -> Result<Self, DatabaseError> {
        let contents = std::fs::read_to_string(path).map_err(|source| DatabaseError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&contents, &path.display().to_string())
    }

    fn parse(toml_text: &str, origin: &str) -> Result<Self, DatabaseError> {
        let file: EngineFile = toml::from_str(toml_text).map_err(|e| DatabaseError::Invalid {
            origin: origin.to_string(),
            message: e.to_string(),
        })?;
        Ok(Self {
            engines: file.engine,
        })
    }

    /// Get an engine by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&Engine> {
        let name_lower = name.to_lowercase();
        self.engines
            .iter()
            .find(|e| e.name().to_lowercase() == name_lower)
    }

    /// List all available engines.
    pub fn list(&self) -> &[Engine] {
        &self.engines
    }

    /// Get available engine names.
    pub fn names(&self) -> Vec<&str> {
        self.engines.iter().map(|e| e.name()).collect()
    }

    /// Suggest similar engine names for a typo.
    /// Returns up to 3 suggestions sorted by similarity.
    pub fn suggest(&self, query: &str) -> Vec<&str> {
        let query_lower = query.to_lowercase();
        let mut scored: Vec<_> = self
            .engines
            .iter()
            .map(|e| {
                let name_lower = e.name().to_lowercase();

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

                (e.name(), score)
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
    /// A copy of the [built-in database](EngineDatabase::builtin).
    fn default() -> Self {
        Self::builtin().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_embedded_database() {
        // Every engine passes Engine::new's checks, or this fails.
        let db = EngineDatabase::load_embedded().unwrap();
        assert_eq!(db.engines.len(), 11);
        assert_eq!(db.engines, EngineDatabase::builtin().engines);
    }

    #[test]
    fn impossible_engines_are_rejected_on_load() {
        let bad = r#"
            [[engine]]
            name = "Backwards"
            thrust_sl = 1000000
            thrust_vac = 900000
            isp_sl = 300
            isp_vac = 320
            dry_mass = 500
            propellant = "LoxRp1"
        "#;
        let err = EngineDatabase::parse(bad, "test").unwrap_err();
        let message = err.to_string();
        assert!(message.contains("Backwards"), "{message}");
        assert!(message.contains("exceeds vacuum"), "{message}");
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let err = EngineDatabase::load_from_file(Path::new("/no/such/engines.toml")).unwrap_err();
        assert!(matches!(err, DatabaseError::Io { .. }));
    }

    #[test]
    fn get_engine_by_name() {
        let db = EngineDatabase::default();

        let merlin = db.get("Merlin-1D").unwrap();
        assert_eq!(merlin.name(), "Merlin-1D");

        let raptor = db.get("raptor-2").unwrap(); // case-insensitive
        assert_eq!(raptor.name(), "Raptor-2");
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
