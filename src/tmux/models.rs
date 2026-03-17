use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxSession {
    pub id: String,
    pub name: String,
    pub attached: bool,
    pub windows: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxWindow {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxPane {
    pub id: String,
    pub window_id: String,
    pub active: bool,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecution {
    pub id: String,
    pub pane_id: String,
    pub command: String,
    pub status: CommandStatus,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub result: Option<String>,
    pub exit_code: Option<i32>,
    pub raw_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommandStatus {
    Pending,
    Completed,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ShellType {
    #[default]
    Bash,
    Zsh,
    Fish,
}

impl ShellType {
    #[allow(dead_code)]
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "zsh" => ShellType::Zsh,
            "fish" => ShellType::Fish,
            _ => ShellType::Bash,
        }
    }

    pub fn exit_code_var(&self) -> &'static str {
        match self {
            ShellType::Fish => "$status",
            _ => "$?",
        }
    }
}
