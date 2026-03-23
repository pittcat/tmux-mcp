use crate::error::{Result, TmuxMcpError};
use crate::tmux::models::{TmuxPane, TmuxSession, TmuxWindow};

pub fn parse_sessions(output: &str) -> Vec<TmuxSession> {
    if output.trim().is_empty() {
        return vec![];
    }

    output
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 {
                Some(TmuxSession {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    attached: parts[2] == "1",
                    windows: parts[3].parse().unwrap_or(0),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn parse_windows(output: &str, session_id: &str) -> Vec<TmuxWindow> {
    if output.trim().is_empty() {
        return vec![];
    }

    let session_id_owned = session_id.to_string();
    output
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                Some(TmuxWindow {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    active: parts[2] == "1",
                    session_id: session_id_owned.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn parse_panes(output: &str, window_id: &str) -> Vec<TmuxPane> {
    if output.trim().is_empty() {
        return vec![];
    }

    let window_id_owned = window_id.to_string();
    output
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                Some(TmuxPane {
                    id: parts[0].to_string(),
                    title: parts[1].to_string(),
                    active: parts[2] == "1",
                    window_id: window_id_owned.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn parse_pane(output: &str) -> Result<TmuxPane> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .find_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 {
                Some(TmuxPane {
                    id: parts[0].to_string(),
                    title: parts[1].to_string(),
                    active: parts[2] == "1",
                    window_id: parts[3].to_string(),
                })
            } else {
                None
            }
        })
        .ok_or_else(|| TmuxMcpError::TmuxError("Failed to parse pane output".to_string()))
}

pub fn parse_command_output(
    content: &str,
    start_marker: &str,
    end_marker_prefix: &str,
) -> Result<(String, i32)> {
    let start_index = content
        .rfind(start_marker)
        .ok_or_else(|| TmuxMcpError::CommandExecutionError("Start marker not found".to_string()))?;

    let end_index = content
        .rfind(end_marker_prefix)
        .ok_or_else(|| TmuxMcpError::CommandExecutionError("End marker not found".to_string()))?;

    if end_index <= start_index {
        return Err(TmuxMcpError::CommandExecutionError(
            "Invalid marker order".to_string(),
        ));
    }

    let end_line_start = end_index;
    let end_line_end = content[end_line_start..]
        .find('\n')
        .map(|i| end_line_start + i)
        .unwrap_or(content.len());
    let end_line = &content[end_line_start..end_line_end];

    let exit_code = end_line
        .strip_prefix(end_marker_prefix)
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| {
            TmuxMcpError::CommandExecutionError("Failed to parse exit code".to_string())
        })?;

    let output_start = start_index + start_marker.len();
    let output_content = &content[output_start..end_index];

    let result = output_content
        .find('\n')
        .map(|i| output_content[i + 1..].trim().to_string())
        .unwrap_or_else(|| output_content.trim().to_string());

    Ok((result, exit_code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sessions() {
        let output = "$0:session1:1:3\n$1:session2:0:1";
        let sessions = parse_sessions(output);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, "$0");
        assert_eq!(sessions[0].name, "session1");
        assert!(sessions[0].attached);
        assert_eq!(sessions[0].windows, 3);
    }

    #[test]
    fn test_parse_windows() {
        let output = "@0:window1:1\n@1:window2:0";
        let windows = parse_windows(output, "$0");
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].id, "@0");
        assert_eq!(windows[0].name, "window1");
        assert!(windows[0].active);
    }

    #[test]
    fn test_parse_panes() {
        let output = "%0:title1:1\n%1:title2:0";
        let panes = parse_panes(output, "@0");
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].id, "%0");
        assert_eq!(panes[0].title, "title1");
        assert!(panes[0].active);
    }

    #[test]
    fn test_parse_pane_from_control_mode_output() {
        let output = "%begin 1 2 1\n%4:title4:1:@2\n%end 1 2 1";
        let pane = parse_pane(output).unwrap();
        assert_eq!(pane.id, "%4");
        assert_eq!(pane.title, "title4");
        assert!(pane.active);
        assert_eq!(pane.window_id, "@2");
    }

    #[test]
    fn test_parse_command_output() {
        let content = "TMUX_MCP_START\necho hello\nhello\nTMUX_MCP_DONE_0\n";
        let (output, exit_code) =
            parse_command_output(content, "TMUX_MCP_START", "TMUX_MCP_DONE_").unwrap();
        assert_eq!(exit_code, 0);
        // Output includes the command and its result
        assert!(output.contains("hello"));
    }
}
