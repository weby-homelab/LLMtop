use super::{AgentCollector, SharedProcessData};
use crate::model::{AgentSession, SessionStatus};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct OllamaCollector {
    addr: String,
}

impl OllamaCollector {
    pub fn new() -> Self {
        Self {
            addr: "127.0.0.1:11434".to_string(),
        }
    }
}

impl Default for OllamaCollector {
    fn default() -> Self {
        Self::new()
    }
}

// Simple HTTP client implementation using std::net::TcpStream
pub fn http_get(addr: &str, path: &str) -> Option<String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let mut stream = TcpStream::connect(addr).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).ok();
    stream.set_write_timeout(Some(Duration::from_millis(500))).ok();

    let request = format!(
        "GET {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Connection: close\r\n\
         Accept: application/json\r\n\r\n",
        path, addr
    );
    stream.write_all(request.as_bytes()).ok();

    let mut response = String::new();
    stream.read_to_string(&mut response).ok();

    if let Some((_, body)) = response.split_once("\r\n\r\n") {
        Some(body.to_string())
    } else {
        Some(response)
    }
}

impl AgentCollector for OllamaCollector {
    fn collect(&mut self, shared: &SharedProcessData) -> Vec<AgentSession> {
        let mut sessions = Vec::new();

        // 1. Query Ollama /api/ps
        let response_str = match http_get(&self.addr, "/api/ps") {
            Some(res) => res,
            None => return sessions, // Ollama not running or unreachable
        };

        let parsed: Value = match serde_json::from_str(&response_str) {
            Ok(v) => v,
            Err(_) => return sessions,
        };

        let models = match parsed["models"].as_array() {
            Some(arr) => arr,
            None => return sessions,
        };

        // Find Ollama PIDs from the process table to map them
        let ollama_pids: Vec<u32> = shared
            .process_info
            .iter()
            .filter(|(_, proc)| {
                super::process::cmd_has_binary(&proc.command, "ollama")
            })
            .map(|(&pid, _)| pid)
            .collect();

        let primary_pid = ollama_pids.first().copied().unwrap_or(0);

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        for m in models {
            let model_name = m["name"].as_str().unwrap_or("unknown").to_string();
            let size = m["size"].as_u64().unwrap_or(0);
            let size_vram = m["size_vram"].as_u64().unwrap_or(0);
            let expires_at = m["expires_at"].as_str().unwrap_or("");

            // Determine processor/GPU status
            let is_gpu = size_vram > 0;
            let gpu_percent = if size > 0 {
                (size_vram as f64 / size as f64) * 100.0
            } else {
                0.0
            };

            // Estimate model parameters and quantization
            let details = &m["details"];
            let family = details["family"].as_str().unwrap_or("");
            let param_size = details["parameter_size"].as_str().unwrap_or("");
            let quant = details["quantization_level"].as_str().unwrap_or("");

            let model_desc = if !param_size.is_empty() {
                format!("{}:{} ({})", family, param_size, quant)
            } else {
                family.to_string()
            };

            // Status: if process has high CPU or we are generating
            // For now, simple waiting/thinking based on CPU usage of ollama PIDs
            let mut is_thinking = false;
            for &pid in &ollama_pids {
                if let Some(proc) = shared.process_info.get(&pid) {
                    if proc.cpu_pct > 10.0 {
                        is_thinking = true;
                        break;
                    }
                }
            }

            let status = if is_thinking {
                SessionStatus::Thinking
            } else {
                SessionStatus::Waiting
            };

            // Formulate prompt description (Expires in ...)
            let initial_prompt = if !expires_at.is_empty() {
                format!("Expires: {}", expires_at.split('T').next().unwrap_or(expires_at))
            } else {
                "Loaded in memory".to_string()
            };

            let mem_mb = size_vram / 1024 / 1024;

            sessions.push(AgentSession {
                agent_cli: "ollama",
                pid: primary_pid,
                session_id: model_name.clone(),
                cwd: "/".to_string(),
                project_name: "Ollama".to_string(),
                started_at: now_ms,
                status,
                model: model_desc,
                effort: if is_gpu { format!("{:.0}% GPU", gpu_percent) } else { "CPU only".to_string() },
                context_percent: 0.0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cache_read: 0,
                total_cache_create: 0,
                turn_count: 1,
                current_tasks: vec![format!("VRAM: {} MB / Total: {} MB", size_vram / 1024 / 1024, size / 1024 / 1024)],
                mem_mb,
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
                initial_prompt,
                first_assistant_text: String::new(),
                chat_messages: vec![],
                tool_calls: vec![],
                pending_since_ms: 0,
                thinking_since_ms: 0,
                file_accesses: vec![],
                config_root: "~/.ollama".to_string(),
            });
        }

        sessions
    }
}
