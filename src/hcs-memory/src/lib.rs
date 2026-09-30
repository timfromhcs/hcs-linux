use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Memory not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryClass {
    Working,
    Session,
    Episodic,
    Semantic,
    Preference,
    Skill,
    Outcome,
    Failure,
    Project,
    Training,
}

impl MemoryClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Session => "session",
            Self::Episodic => "episodic",
            Self::Semantic => "semantic",
            Self::Preference => "preference",
            Self::Skill => "skill",
            Self::Outcome => "outcome",
            Self::Failure => "failure",
            Self::Project => "project",
            Self::Training => "training",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "working" => Some(Self::Working),
            "session" => Some(Self::Session),
            "episodic" => Some(Self::Episodic),
            "semantic" => Some(Self::Semantic),
            "preference" => Some(Self::Preference),
            "skill" => Some(Self::Skill),
            "outcome" => Some(Self::Outcome),
            "failure" => Some(Self::Failure),
            "project" => Some(Self::Project),
            "training" => Some(Self::Training),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationState {
    Raw,
    Candidate,
    Verified,
    Promoted,
    Superseded,
}

impl VerificationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Candidate => "candidate",
            Self::Verified => "verified",
            Self::Promoted => "promoted",
            Self::Superseded => "superseded",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "raw" => Some(Self::Raw),
            "candidate" => Some(Self::Candidate),
            "verified" => Some(Self::Verified),
            "promoted" => Some(Self::Promoted),
            "superseded" => Some(Self::Superseded),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub memory_class: MemoryClass,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source: String,
    pub content: String,
    pub confidence: f64,
    pub verification_state: VerificationState,
    pub last_verified: Option<DateTime<Utc>>,
    pub project: Option<String>,
    pub privacy_class: String,
    pub hash: String,
    pub superseded_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    pub record: MemoryRecord,
    pub lexical_score: f64,
    pub semantic_score: f64,
    pub recency_score: f64,
    pub total_score: f64,
}

pub struct MemoryEngine {
    conn: Connection,
}

impl MemoryEngine {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, MemoryError> {
        let conn = Connection::open(path)?;
        let engine = Self { conn };
        engine.init_schema()?;
        Ok(engine)
    }

    pub fn open_in_memory() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory()?;
        let engine = Self { conn };
        engine.init_schema()?;
        Ok(engine)
    }

    fn init_schema(&self) -> Result<(), MemoryError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                memory_class TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                source TEXT NOT NULL,
                content TEXT NOT NULL,
                confidence REAL NOT NULL,
                verification_state TEXT NOT NULL,
                last_verified TEXT,
                project TEXT,
                privacy_class TEXT NOT NULL,
                hash TEXT NOT NULL,
                superseded_by TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_memories_class ON memories(memory_class);
            CREATE INDEX IF NOT EXISTS idx_memories_hash ON memories(hash);
            CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project);

            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                content,
                source,
                content='memories',
                content_rowid='rowid'
            );

            CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
                INSERT INTO memories_fts(rowid, content, source) VALUES (new.rowid, new.content, new.source);
            END;

            CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, content, source) VALUES('delete', old.rowid, old.content, old.source);
            END;

            CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, content, source) VALUES('delete', old.rowid, old.content, old.source);
                INSERT INTO memories_fts(rowid, content, source) VALUES (new.rowid, new.content, new.source);
            END;
            "#,
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert(
        &mut self,
        memory_class: MemoryClass,
        source: &str,
        content: &str,
        confidence: f64,
        verification_state: VerificationState,
        project: Option<String>,
        privacy_class: &str,
    ) -> Result<MemoryRecord, MemoryError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hex::encode(hasher.finalize());

        let last_verified = if verification_state == VerificationState::Verified
            || verification_state == VerificationState::Promoted
        {
            Some(now)
        } else {
            None
        };

        self.conn.execute(
            r#"
            INSERT INTO memories (
                id, memory_class, created_at, updated_at, source, content,
                confidence, verification_state, last_verified, project,
                privacy_class, hash, superseded_by
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                id,
                memory_class.as_str(),
                now.to_rfc3339(),
                now.to_rfc3339(),
                source,
                content,
                confidence,
                verification_state.as_str(),
                last_verified.map(|t| t.to_rfc3339()),
                project,
                privacy_class,
                hash,
                None::<String>,
            ],
        )?;

        Ok(MemoryRecord {
            id,
            memory_class,
            created_at: now,
            updated_at: now,
            source: source.to_string(),
            content: content.to_string(),
            confidence,
            verification_state,
            last_verified,
            project,
            privacy_class: privacy_class.to_string(),
            hash,
            superseded_by: None,
        })
    }

    pub fn get_by_id(&self, id: &str) -> Result<MemoryRecord, MemoryError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, memory_class, created_at, updated_at, source, content,
                   confidence, verification_state, last_verified, project,
                   privacy_class, hash, superseded_by
            FROM memories WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let class_str: String = row.get(1)?;
            let vstate_str: String = row.get(7)?;
            let created_str: String = row.get(2)?;
            let updated_str: String = row.get(3)?;
            let last_v_str: Option<String> = row.get(8)?;

            Ok(MemoryRecord {
                id: row.get(0)?,
                memory_class: MemoryClass::from_str_opt(&class_str).unwrap_or(MemoryClass::Working),
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&updated_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                source: row.get(4)?,
                content: row.get(5)?,
                confidence: row.get(6)?,
                verification_state: VerificationState::from_str_opt(&vstate_str)
                    .unwrap_or(VerificationState::Raw),
                last_verified: last_v_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
                project: row.get(9)?,
                privacy_class: row.get(10)?,
                hash: row.get(11)?,
                superseded_by: row.get(12)?,
            })
        } else {
            Err(MemoryError::NotFound(id.to_string()))
        }
    }

    pub fn hybrid_search(
        &self,
        query: &str,
        project_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ScoredMemory>, MemoryError> {
        // Query FTS5 lexical matching first
        let mut stmt = self.conn.prepare(
            r#"
            SELECT m.id, m.memory_class, m.created_at, m.updated_at, m.source, m.content,
                   m.confidence, m.verification_state, m.last_verified, m.project,
                   m.privacy_class, m.hash, m.superseded_by,
                   bm25(memories_fts) as rank
            FROM memories_fts f
            JOIN memories m ON m.rowid = f.rowid
            WHERE memories_fts MATCH ?1 AND m.verification_state != 'superseded'
            ORDER BY rank
            LIMIT ?2
            "#,
        )?;

        // Clean query terms for FTS5 (avoid special characters breaking query)
        let sanitized_query: String = query
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c.is_whitespace() {
                    c
                } else {
                    ' '
                }
            })
            .collect();
        let terms: Vec<&str> = sanitized_query.split_whitespace().collect();
        let fts_query = if terms.is_empty() {
            query.to_string()
        } else {
            terms.join(" OR ")
        };

        let mut scored_memories = Vec::new();
        let mut rows = match stmt.query(params![fts_query, (limit * 2) as i64]) {
            Ok(r) => r,
            Err(_) => {
                // Fallback to substring query if FTS expression parsing fails
                return self.fallback_search(query, project_filter, limit);
            }
        };

        let now = Utc::now();
        while let Some(row) = rows.next()? {
            let class_str: String = row.get(1)?;
            let vstate_str: String = row.get(7)?;
            let created_str: String = row.get(2)?;
            let updated_str: String = row.get(3)?;
            let last_v_str: Option<String> = row.get(8)?;
            let project: Option<String> = row.get(9)?;

            if let Some(pf) = project_filter {
                if project.as_deref() != Some(pf) && project.is_some() {
                    continue;
                }
            }

            let rank: f64 = row.get(13)?;
            let lexical_score = (1.0 / (1.0 + rank.abs())).clamp(0.0, 1.0);

            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(now);
            let age_hours = (now - created_at).num_hours().max(0) as f64;
            let recency_score = 1.0 / (1.0 + (age_hours / 24.0)); // decays over days

            let confidence: f64 = row.get(6)?;
            let semantic_score = 0.5; // Baseline when embedding model is offline

            let total_score = (0.40 * lexical_score)
                + (0.30 * semantic_score)
                + (0.15 * recency_score)
                + (0.15 * confidence);

            scored_memories.push(ScoredMemory {
                record: MemoryRecord {
                    id: row.get(0)?,
                    memory_class: MemoryClass::from_str_opt(&class_str)
                        .unwrap_or(MemoryClass::Working),
                    created_at,
                    updated_at: DateTime::parse_from_rfc3339(&updated_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(now),
                    source: row.get(4)?,
                    content: row.get(5)?,
                    confidence,
                    verification_state: VerificationState::from_str_opt(&vstate_str)
                        .unwrap_or(VerificationState::Raw),
                    last_verified: last_v_str.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    project,
                    privacy_class: row.get(10)?,
                    hash: row.get(11)?,
                    superseded_by: row.get(12)?,
                },
                lexical_score,
                semantic_score,
                recency_score,
                total_score,
            });

            if scored_memories.len() >= limit {
                break;
            }
        }

        scored_memories.sort_by(|a, b| {
            b.total_score
                .partial_cmp(&a.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(scored_memories)
    }

    fn fallback_search(
        &self,
        query: &str,
        project_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ScoredMemory>, MemoryError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, memory_class, created_at, updated_at, source, content,
                   confidence, verification_state, last_verified, project,
                   privacy_class, hash, superseded_by
            FROM memories
            WHERE content LIKE ?1 AND verification_state != 'superseded'
            LIMIT ?2
            "#,
        )?;

        let pattern = format!("%{}%", query);
        let mut rows = stmt.query(params![pattern, limit as i64])?;
        let mut results = Vec::new();
        let now = Utc::now();

        while let Some(row) = rows.next()? {
            let class_str: String = row.get(1)?;
            let vstate_str: String = row.get(7)?;
            let created_str: String = row.get(2)?;
            let updated_str: String = row.get(3)?;
            let last_v_str: Option<String> = row.get(8)?;
            let project: Option<String> = row.get(9)?;

            if let Some(pf) = project_filter {
                if project.as_deref() != Some(pf) && project.is_some() {
                    continue;
                }
            }

            let confidence: f64 = row.get(6)?;
            results.push(ScoredMemory {
                record: MemoryRecord {
                    id: row.get(0)?,
                    memory_class: MemoryClass::from_str_opt(&class_str)
                        .unwrap_or(MemoryClass::Working),
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(now),
                    updated_at: DateTime::parse_from_rfc3339(&updated_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(now),
                    source: row.get(4)?,
                    content: row.get(5)?,
                    confidence,
                    verification_state: VerificationState::from_str_opt(&vstate_str)
                        .unwrap_or(VerificationState::Raw),
                    last_verified: last_v_str.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    project,
                    privacy_class: row.get(10)?,
                    hash: row.get(11)?,
                    superseded_by: row.get(12)?,
                },
                lexical_score: 0.8,
                semantic_score: 0.5,
                recency_score: 0.8,
                total_score: 0.7,
            });
        }
        Ok(results)
    }

    pub fn promote_candidate(&mut self, id: &str) -> Result<bool, MemoryError> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            r#"
            UPDATE memories
            SET verification_state = 'promoted', last_verified = ?1, updated_at = ?1
            WHERE id = ?2 AND verification_state IN ('candidate', 'verified')
            "#,
            params![now, id],
        )?;
        Ok(rows > 0)
    }

    pub fn consolidate_memories(&mut self) -> Result<usize, MemoryError> {
        // Mark duplicates with identical hash as superseded by the newest verified record
        let mut stmt = self.conn.prepare(
            r#"
            SELECT hash, COUNT(*) as cnt
            FROM memories
            WHERE verification_state != 'superseded'
            GROUP BY hash
            HAVING cnt > 1
            "#,
        )?;

        let duplicate_hashes: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();

        let mut consolidated_count = 0;
        let now = Utc::now().to_rfc3339();

        for hash in duplicate_hashes {
            let mut records_stmt = self.conn.prepare(
                r#"
                SELECT id, verification_state
                FROM memories
                WHERE hash = ?1 AND verification_state != 'superseded'
                ORDER BY created_at DESC
                "#,
            )?;

            let records: Vec<(String, String)> = records_stmt
                .query_map(params![hash], |row| Ok((row.get(0)?, row.get(1)?)))?
                .filter_map(Result::ok)
                .collect();

            if records.len() > 1 {
                let keep_id = &records[0].0;
                for (other_id, _) in &records[1..] {
                    self.conn.execute(
                        r#"
                        UPDATE memories
                        SET verification_state = 'superseded', superseded_by = ?1, updated_at = ?2
                        WHERE id = ?3
                        "#,
                        params![keep_id, now, other_id],
                    )?;
                    consolidated_count += 1;
                }
            }
        }

        Ok(consolidated_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_insert_and_retrieve() {
        let mut engine = MemoryEngine::open_in_memory().unwrap();
        let record = engine
            .insert(
                MemoryClass::Skill,
                "test_suite",
                "Building HCS Linux requires live-build and debian trixie",
                0.95,
                VerificationState::Verified,
                Some("hcs-linux".to_string()),
                "public",
            )
            .unwrap();

        let retrieved = engine.get_by_id(&record.id).unwrap();
        assert_eq!(retrieved.content, record.content);
        assert_eq!(retrieved.verification_state, VerificationState::Verified);
        assert_eq!(retrieved.memory_class, MemoryClass::Skill);
    }

    #[test]
    fn test_hybrid_search() {
        let mut engine = MemoryEngine::open_in_memory().unwrap();
        engine
            .insert(
                MemoryClass::Semantic,
                "manual",
                "Wayland compositor niri uses scrollable tiling",
                0.90,
                VerificationState::Promoted,
                None,
                "public",
            )
            .unwrap();

        engine
            .insert(
                MemoryClass::Episodic,
                "task",
                "Kernel parameters configured with zram swap",
                0.85,
                VerificationState::Candidate,
                None,
                "public",
            )
            .unwrap();

        let results = engine.hybrid_search("niri tiling", None, 5).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].record.content.contains("niri"));
    }

    #[test]
    fn test_consolidation() {
        let mut engine = MemoryEngine::open_in_memory().unwrap();
        let c1 = "Identical lesson learned twice";
        engine
            .insert(
                MemoryClass::Skill,
                "task1",
                c1,
                0.9,
                VerificationState::Candidate,
                None,
                "public",
            )
            .unwrap();
        engine
            .insert(
                MemoryClass::Skill,
                "task2",
                c1,
                0.95,
                VerificationState::Verified,
                None,
                "public",
            )
            .unwrap();

        let consolidated = engine.consolidate_memories().unwrap();
        assert_eq!(consolidated, 1);
    }
}
