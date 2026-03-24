use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub max_commands: usize,
    pub command_ttl_seconds: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind_addr =
            std::env::var("TMUX_MCP_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8090".to_string());

        let max_commands = std::env::var("TMUX_MCP_MAX_COMMANDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        let command_ttl_seconds = std::env::var("TMUX_MCP_COMMAND_TTL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(600);

        Ok(Config {
            bind_addr,
            max_commands,
            command_ttl_seconds,
        })
    }
}
