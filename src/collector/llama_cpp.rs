use super::{AgentCollector, SharedProcessData};
use crate::model::{AgentSession, SessionStatus};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct LlamaCppCollector {
    addr: String,
}

impl LlamaCppCollector {
    pub fn new() -> Self {
        Self {
            addr: "127.0.0.1:8080".to_string(),
        }
    }
}

impl Default for LlamaCppCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentCollector for LlamaCppCollector {
    fn collect(&mut self, shared: &SharedProcessData) -> Vec<AgentSession> {
        let mut sessions = Vec::new();

        // 1. Query Llama.cpp /health
        let health_str = match super::ollama::http_get(&self.addr, "/health") {
            Some(res) => res,
            None => return sessions,
        };

        let _health_val: Value = match serde_json::from_str(&health_str) {
            Ok(v) => v,
            Err(_) => return sessions,
        };

        // 2. Query slots info for active work
        let slots_str = super::ollama::http_get(&self.addr, "/slots").unwrap_or_default();
        let slots_val: Value = serde_json::from_str(&slots_str).unwrap_or(Value::Null);

        // Find llama-server PID
        let llama_pids: Vec<u32> = shared
            .process_info
            .iter()
            .filter(|(_, proc)| {
                super::process::cmd_has_binary(&proc.command, "llama-server")
                    || super::process::cmd_has_binary(&proc.command, "llama")
            })
            .map(|(&pid, _)| pid)
            .collect();

        let primary_pid = llama_pids.first().copied().unwrap_or(0);
        let rss_kb = if primary_pid != 0 {
            shared.process_info.get(&primary_pid).map(|p| p.rss_kb).unwrap_or(0)
        } else {
            0
        };

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Collect stats from slots
        let mut active_slots = 0;
        let mut total_slots = 0;
        let mut prompt_tokens = 0;
        let mut gen_tokens = 0;

        if let Some(arr) = slots_val.as_array() {
            total_slots = arr.len();
            for slot in arr {
                let state = slot["state"].as_u64().unwrap_or(0); // 0=idle, 1=processing
                if state == 1 || slot["is_processing"].as_bool() == Some(true) {
                    active_slots += 1;
                }
                prompt_tokens += slot["n_past"].as_u64().unwrap_or(0);
                gen_tokens += slot["n_decoded"].as_u64().unwrap_or(0);
            }
        }

        let is_thinking = active_slots > 0 || llama_pids.iter().any(|&pid| {
            shared.process_info.get(&pid).map(|p| p.cpu_pct > 10.0).unwrap_or(false)
        });

        let status = if is_thinking {
            SessionStatus::Thinking
        } else {
            SessionStatus::Waiting
        };

        // Determine loaded model name
        let mut model_name = "llama-server".to_string();
        if let Some(&pid) = llama_pids.first() {
            if let Some(proc) = shared.process_info.get(&pid) {
                let parts: Vec<&str> = proc.command.split_whitespace().collect();
                if let Some(idx) = parts.iter().position(|&r| r == "-m" || r == "--model") {
                    if let Some(path) = parts.get(idx + 1) {
                        if let Some(filename) = path.rsplit('/').next() {
                            model_name = filename.to_string();
                        }
                    }
                }
            }
        }

        sessions.push(AgentSession {
            agent_cli: "llama.cpp",
            pid: primary_pid,
            session_id: "llama-server".to_string(),
            cwd: "/".to_string(),
            project_name: "Llama.cpp".to_string(),
            started_at: now_ms,
            status,
            model: model_name,
            effort: format!("{}/{} slots", active_slots, total_slots),
            context_percent: 0.0,
            total_input_tokens: prompt_tokens,
            total_output_tokens: gen_tokens,
            total_cache_read: 0,
            total_cache_create: 0,
            turn_count: active_slots as u32,
            current_tasks: vec![format!("Active slots: {} / Total slots: {}", active_slots, total_slots)],
            mem_mb: rss_kb / 1024,
            version: String::new(),
            git_branch: String::new(),
            git_added: 0,
            git_modified: 0,
            token_history: vec![],
            context_history: vec![],
            compaction_count: 0,
            context_window: 8192,
            subagents: vec![],
            mem_file_count: 0,
            mem_line_count: 0,
            children: vec![],
            initial_prompt: format!("Slots active: {} / Total: {}", active_slots, total_slots),
            first_assistant_text: String::new(),
            chat_messages: vec![],
            tool_calls: vec![],
            pending_since_ms: 0,
            thinking_since_ms: 0,
            file_accesses: vec![],
            config_root: "llama-server".to_string(),
        });

        sessions
    }
}
