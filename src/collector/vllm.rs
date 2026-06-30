use super::{AgentCollector, SharedProcessData};
use crate::model::{AgentSession, SessionStatus};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct VllmCollector {
    addr: String,
}

impl VllmCollector {
    pub fn new() -> Self {
        Self {
            addr: "127.0.0.1:8000".to_string(),
        }
    }
}

impl Default for VllmCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentCollector for VllmCollector {
    fn collect(&mut self, shared: &SharedProcessData) -> Vec<AgentSession> {
        let mut sessions = Vec::new();

        // Query vLLM /metrics
        let metrics_str = match super::ollama::http_get(&self.addr, "/metrics") {
            Some(res) => res,
            None => return sessions,
        };

        // Find vLLM PIDs
        let vllm_pids: Vec<u32> = shared
            .process_info
            .iter()
            .filter(|(_, proc)| {
                proc.command.contains("vllm")
            })
            .map(|(&pid, _)| pid)
            .collect();

        let primary_pid = vllm_pids.first().copied().unwrap_or(0);
        let rss_kb = if primary_pid != 0 {
            shared.process_info.get(&primary_pid).map(|p| p.rss_kb).unwrap_or(0)
        } else {
            0
        };

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut kv_cache_usage = 0.0;
        let mut num_running = 0;
        let mut num_waiting = 0;

        for line in metrics_str.lines() {
            if line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            let name = parts[0];
            let val_str = parts[1];

            let clean_name = name.split('{').next().unwrap_or(name);

            if clean_name.contains("vllm:kv_cache_usage_perc") || clean_name.contains("vllm_kv_cache_usage_perc") {
                kv_cache_usage = val_str.parse::<f64>().unwrap_or(0.0) * 100.0;
            } else if clean_name.contains("vllm:num_requests_running") || clean_name.contains("vllm_num_requests_running") {
                num_running = val_str.parse::<u64>().unwrap_or(0);
            } else if clean_name.contains("vllm:num_requests_waiting") || clean_name.contains("vllm_num_requests_waiting") {
                num_waiting = val_str.parse::<u64>().unwrap_or(0);
            }
        }

        let is_thinking = num_running > 0 || vllm_pids.iter().any(|&pid| {
            shared.process_info.get(&pid).map(|p| p.cpu_pct > 10.0).unwrap_or(false)
        });

        let status = if is_thinking {
            SessionStatus::Thinking
        } else {
            SessionStatus::Waiting
        };

        // Determine loaded model name
        let mut model_name = "vllm-server".to_string();
        if let Some(models_json) = super::ollama::http_get(&self.addr, "/v1/models") {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&models_json) {
                if let Some(arr) = val["data"].as_array() {
                    if let Some(first) = arr.first() {
                        if let Some(id) = first["id"].as_str() {
                            model_name = id.to_string();
                        }
                    }
                }
            }
        }

        sessions.push(AgentSession {
            agent_cli: "vllm",
            pid: primary_pid,
            session_id: "vllm".to_string(),
            cwd: "/".to_string(),
            project_name: "vLLM".to_string(),
            started_at: now_ms,
            status,
            model: model_name,
            effort: format!("KV Cache: {:.1}%", kv_cache_usage),
            context_percent: kv_cache_usage,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_read: 0,
            total_cache_create: 0,
            turn_count: num_running as u32,
            current_tasks: vec![format!("Running reqs: {} / Waiting: {}", num_running, num_waiting)],
            mem_mb: rss_kb / 1024,
            version: String::new(),
            git_branch: String::new(),
            git_added: 0,
            git_modified: 0,
            token_history: vec![],
            context_history: vec![],
            compaction_count: 0,
            context_window: 32768,
            subagents: vec![],
            mem_file_count: 0,
            mem_line_count: 0,
            children: vec![],
            initial_prompt: format!("Running: {} | Waiting: {}", num_running, num_waiting),
            first_assistant_text: String::new(),
            chat_messages: vec![],
            tool_calls: vec![],
            pending_since_ms: 0,
            thinking_since_ms: 0,
            file_accesses: vec![],
            config_root: "vllm".to_string(),
        });

        sessions
    }
}
