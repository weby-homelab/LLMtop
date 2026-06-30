use super::{AgentCollector, SharedProcessData};
use crate::model::{AgentSession, SessionStatus};
use std::time::{SystemTime, UNIX_EPOCH};

/// Universal OpenAI-compatible API auto-discovery collector.
///
/// On each slow tick, scans local listening ports from the shared process data
/// and probes each for an OpenAI-compatible `/v1/models` endpoint. Any server
/// that responds with valid JSON is surfaced as a session, enabling llm-top to
/// automatically detect:
///
/// - LM Studio
/// - LiteLLM
/// - Open WebUI
/// - LocalAI
/// - KoboldCpp
/// - TabbyAPI
/// - Hermes Agent (when exposing an API)
/// - text-generation-webui (oobabooga)
/// - Any other OpenAI-compatible inference server
///
/// Ports already claimed by known collectors (Ollama 11434, llama.cpp 8080,
/// vLLM 8000) are skipped to avoid duplicate sessions.
pub struct AutoDiscoverCollector {
    /// Cached discovered servers from the last scan.
    cached_servers: Vec<DiscoveredServer>,
    /// Ports that are already handled by dedicated collectors.
    known_ports: Vec<u16>,
}

struct DiscoveredServer {
    port: u16,
    pid: u32,
    server_name: String,
    models: Vec<String>,
    rss_kb: u64,
}

impl AutoDiscoverCollector {
    pub fn new() -> Self {
        Self {
            cached_servers: Vec::new(),
            known_ports: vec![11434, 8080, 8000], // Ollama, llama.cpp, vLLM defaults
        }
    }
}

impl Default for AutoDiscoverCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Identify the server software from its HTTP response headers or /v1/models body.
fn identify_server(addr: &str, body: &str) -> String {
    // Try to get the Server header from a raw HTTP response
    if let Some(raw) = raw_http_get(addr, "/v1/models") {
        let lower = raw.to_lowercase();
        if lower.contains("litellm") {
            return "LiteLLM".to_string();
        }
        if lower.contains("lm-studio") || lower.contains("lm studio") {
            return "LM Studio".to_string();
        }
        if lower.contains("localai") {
            return "LocalAI".to_string();
        }
        if lower.contains("koboldcpp") || lower.contains("kobold") {
            return "KoboldCpp".to_string();
        }
        if lower.contains("tabbyapi") || lower.contains("tabby") {
            return "TabbyAPI".to_string();
        }
        if lower.contains("open-webui") || lower.contains("openwebui") {
            return "Open WebUI".to_string();
        }
        if lower.contains("text-generation") || lower.contains("oobabooga") {
            return "TGW".to_string();
        }
        if lower.contains("hermes") {
            return "Hermes".to_string();
        }
        if lower.contains("jan") {
            return "Jan".to_string();
        }
        if lower.contains("odysseus") {
            return "Odysseus".to_string();
        }
    }

    // Fallback: try to identify from the JSON body structure
    let body_lower = body.to_lowercase();
    if body_lower.contains("litellm") {
        return "LiteLLM".to_string();
    }
    if body_lower.contains("kobold") {
        return "KoboldCpp".to_string();
    }

    "OpenAI-API".to_string()
}

/// Raw HTTP GET that returns the full response (headers + body).
fn raw_http_get(addr: &str, path: &str) -> Option<String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let mut stream = TcpStream::connect(addr).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_millis(300)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_millis(300)))
        .ok();

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
    if response.is_empty() {
        None
    } else {
        Some(response)
    }
}

/// Extract just the body from a raw HTTP response.
fn extract_body(raw: &str) -> Option<&str> {
    raw.split_once("\r\n\r\n").map(|(_, body)| body)
}

/// Parse model IDs from an OpenAI `/v1/models` response.
fn parse_model_ids(body: &str) -> Vec<String> {
    // Response format: {"data": [{"id": "model-name", ...}, ...]}
    let val: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut models = Vec::new();
    if let Some(arr) = val["data"].as_array() {
        for item in arr {
            if let Some(id) = item["id"].as_str() {
                models.push(id.to_string());
            }
        }
    }
    // Some servers use "models" instead of "data"
    if models.is_empty() {
        if let Some(arr) = val["models"].as_array() {
            for item in arr {
                if let Some(id) = item["id"].as_str() {
                    models.push(id.to_string());
                } else if let Some(name) = item["name"].as_str() {
                    models.push(name.to_string());
                }
            }
        }
    }
    models
}

impl AgentCollector for AutoDiscoverCollector {
    fn collect(&mut self, shared: &SharedProcessData) -> Vec<AgentSession> {
        // Only scan on slow ticks to avoid hammering ports every 2s
        if !shared.slow_tick && !self.cached_servers.is_empty() {
            // Return cached results on fast ticks
            return self.build_sessions(shared);
        }

        // Collect all unique listening ports from the shared port map
        let mut port_pids: Vec<(u16, u32)> = Vec::new();
        for (&pid, ports) in &shared.ports {
            for &port in ports {
                // Skip ports claimed by dedicated collectors
                if self.known_ports.contains(&port) {
                    continue;
                }
                // Skip very low ports (system services) and very high ephemeral ports
                if !(1024..=49151).contains(&port) {
                    continue;
                }
                port_pids.push((port, pid));
            }
        }
        port_pids.sort_by_key(|&(port, _)| port);
        port_pids.dedup_by_key(|pp| pp.0);

        let mut discovered = Vec::new();

        for (port, pid) in port_pids {
            let addr = format!("127.0.0.1:{}", port);

            // Probe /v1/models
            let raw = match raw_http_get(&addr, "/v1/models") {
                Some(r) => r,
                None => continue,
            };

            // Must be a successful HTTP response
            if !raw.starts_with("HTTP/1.") || !raw.contains("200") {
                continue;
            }

            let body = match extract_body(&raw) {
                Some(b) => b,
                None => continue,
            };

            let models = parse_model_ids(body);
            if models.is_empty() {
                continue;
            }

            let server_name = identify_server(&addr, body);

            let rss_kb = shared
                .process_info
                .get(&pid)
                .map(|p| p.rss_kb)
                .unwrap_or(0);

            discovered.push(DiscoveredServer {
                port,
                pid,
                server_name,
                models,
                rss_kb,
            });
        }

        self.cached_servers = discovered;
        self.build_sessions(shared)
    }
}

impl AutoDiscoverCollector {
    fn build_sessions(&self, _shared: &SharedProcessData) -> Vec<AgentSession> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut sessions = Vec::new();

        for server in &self.cached_servers {
            let model_str = if server.models.len() == 1 {
                server.models[0].clone()
            } else {
                format!("{} (+{})", server.models[0], server.models.len() - 1)
            };

            let is_active = server.pid != 0;
            let status = if is_active {
                SessionStatus::Waiting
            } else {
                SessionStatus::Unknown
            };

            sessions.push(AgentSession {
                agent_cli: "auto",
                pid: server.pid,
                session_id: format!("{}:{}", server.server_name, server.port),
                cwd: "/".to_string(),
                project_name: server.server_name.clone(),
                started_at: now_ms,
                status,
                model: model_str,
                effort: format!(":{}", server.port),
                context_percent: 0.0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cache_read: 0,
                total_cache_create: 0,
                turn_count: server.models.len() as u32,
                current_tasks: vec![format!(
                    "Port {} | {} model(s)",
                    server.port,
                    server.models.len()
                )],
                mem_mb: server.rss_kb / 1024,
                version: String::new(),
                git_branch: String::new(),
                git_added: 0,
                git_modified: 0,
                token_history: vec![],
                context_history: vec![],
                compaction_count: 0,
                context_window: 0,
                subagents: vec![],
                mem_file_count: 0,
                mem_line_count: 0,
                children: vec![],
                initial_prompt: format!(
                    "{} on port {} ({} models loaded)",
                    server.server_name,
                    server.port,
                    server.models.len()
                ),
                first_assistant_text: String::new(),
                chat_messages: vec![],
                tool_calls: vec![],
                pending_since_ms: 0,
                thinking_since_ms: 0,
                file_accesses: vec![],
                config_root: format!("localhost:{}", server.port),
            });
        }

        sessions
    }
}
