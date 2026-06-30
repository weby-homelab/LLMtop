use super::{AgentCollector, SharedProcessData};
use crate::model::{AgentSession, SessionStatus};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Collector for Odysseus AI Workspace.
/// Reads the SQLite database at `data/app.db` (sessions + chat_messages tables)
/// to surface active chat sessions as llm-top entries.
pub struct OdysseusCollector {
    db_path: PathBuf,
    /// Whether sqlite3 CLI is available (checked once).
    sqlite3_available: Option<bool>,
    /// Cached sessions from the last slow-tick query.
    cached_sessions: Vec<OdySession>,
    /// Last modification time of the DB file.
    last_db_mtime: Option<SystemTime>,
}

struct OdySession {
    id: String,
    title: String,
    model_id: String,
    created_at: u64,
    updated_at: u64,
    message_count: u32,
    total_tokens: u64,
}

impl OdysseusCollector {
    pub fn new() -> Self {
        // Try common Odysseus data paths
        let db_path = find_odysseus_db();
        Self {
            db_path,
            sqlite3_available: None,
            cached_sessions: Vec::new(),
            last_db_mtime: None,
        }
    }
}

impl Default for OdysseusCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn find_odysseus_db() -> PathBuf {
    // Check known paths in priority order
    let candidates = [
        PathBuf::from("/root/odysseus/data/app.db"),
        dirs::home_dir()
            .unwrap_or_default()
            .join("odysseus/data/app.db"),
        dirs::home_dir()
            .unwrap_or_default()
            .join(".odysseus/data/app.db"),
        dirs::data_dir()
            .unwrap_or_default()
            .join("odysseus/app.db"),
    ];
    for p in &candidates {
        if p.exists() {
            return p.clone();
        }
    }
    // Default fallback
    candidates[0].clone()
}

fn check_sqlite3() -> bool {
    std::process::Command::new("sqlite3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn db_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn query_odysseus_sessions(db_path: &Path) -> Vec<OdySession> {
    let sql = r#"
SELECT
  s.id,
  COALESCE(s.title, '(untitled)'),
  COALESCE(s.model_id, 'unknown'),
  COALESCE(CAST(strftime('%s', s.created_at) AS INTEGER), 0),
  COALESCE(CAST(strftime('%s', s.updated_at) AS INTEGER), 0),
  (SELECT COUNT(*) FROM chat_messages cm WHERE cm.session_id = s.id),
  COALESCE(
    (SELECT SUM(
      COALESCE(json_extract(cm.metadata, '$.usage.total_tokens'), 0)
    ) FROM chat_messages cm WHERE cm.session_id = s.id),
    0
  )
FROM sessions s
WHERE s.updated_at >= datetime('now', '-24 hours')
ORDER BY s.updated_at DESC
LIMIT 50;
"#;

    let output = match std::process::Command::new("sqlite3")
        .arg("-separator")
        .arg("|")
        .arg("-readonly")
        .arg(db_path.to_string_lossy().as_ref())
        .arg(sql.trim())
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut sessions = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(7, '|').collect();
        if parts.len() < 7 {
            continue;
        }
        sessions.push(OdySession {
            id: parts[0].to_string(),
            title: parts[1].to_string(),
            model_id: parts[2].to_string(),
            created_at: parts[3].parse().unwrap_or(0),
            updated_at: parts[4].parse().unwrap_or(0),
            message_count: parts[5].parse().unwrap_or(0),
            total_tokens: parts[6].parse().unwrap_or(0),
        });
    }
    sessions
}

impl AgentCollector for OdysseusCollector {
    fn collect(&mut self, shared: &SharedProcessData) -> Vec<AgentSession> {
        // Check if sqlite3 is available (once)
        if self.sqlite3_available.is_none() {
            self.sqlite3_available = Some(check_sqlite3());
        }
        if self.sqlite3_available != Some(true) {
            return Vec::new();
        }

        // Only refresh on slow tick or when DB mtime changes
        let current_mtime = db_mtime(&self.db_path);
        let needs_refresh = shared.slow_tick
            || current_mtime != self.last_db_mtime
            || self.cached_sessions.is_empty();

        if needs_refresh {
            if self.db_path.exists() {
                self.cached_sessions = query_odysseus_sessions(&self.db_path);
                self.last_db_mtime = current_mtime;
            } else {
                return Vec::new();
            }
        }

        // Find Odysseus process (uvicorn with odysseus in cmdline)
        let ody_pid: u32 = shared
            .process_info
            .iter()
            .find(|(_, proc)| {
                proc.command.contains("odysseus")
                    || (proc.command.contains("uvicorn")
                        && proc.command.contains("7000"))
            })
            .map(|(&pid, _)| pid)
            .unwrap_or(0);

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let rss_kb = if ody_pid != 0 {
            shared
                .process_info
                .get(&ody_pid)
                .map(|p| p.rss_kb)
                .unwrap_or(0)
        } else {
            0
        };

        self.cached_sessions
            .iter()
            .map(|s| {
                // Consider "active" if updated within last 5 minutes
                let age_secs = now_ms / 1000 - s.updated_at;
                let status = if age_secs < 300 {
                    SessionStatus::Thinking
                } else if age_secs < 3600 {
                    SessionStatus::Waiting
                } else {
                    SessionStatus::Unknown
                };

                AgentSession {
                    agent_cli: "odysseus",
                    pid: ody_pid,
                    session_id: s.id.clone(),
                    cwd: "/root/odysseus".to_string(),
                    project_name: "Odysseus".to_string(),
                    started_at: s.created_at * 1000,
                    status,
                    model: s.model_id.clone(),
                    effort: format!("{} msgs", s.message_count),
                    context_percent: 0.0,
                    total_input_tokens: s.total_tokens,
                    total_output_tokens: 0,
                    total_cache_read: 0,
                    total_cache_create: 0,
                    turn_count: s.message_count,
                    current_tasks: vec![s.title.clone()],
                    mem_mb: rss_kb / 1024,
                    version: String::new(),
                    git_branch: String::new(),
                    git_added: 0,
                    git_modified: 0,
                    token_history: vec![],
                    context_history: vec![],
                    compaction_count: 0,
                    context_window: 128000,
                    subagents: vec![],
                    mem_file_count: 0,
                    mem_line_count: 0,
                    children: vec![],
                    initial_prompt: s.title.clone(),
                    first_assistant_text: String::new(),
                    chat_messages: vec![],
                    tool_calls: vec![],
                    pending_since_ms: 0,
                    thinking_since_ms: 0,
                    file_accesses: vec![],
                    config_root: "~/odysseus".to_string(),
                }
            })
            .collect()
    }
}
