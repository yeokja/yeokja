use crate::provider::{
    CompletionRequest, CompletionResponse, LlmProvider, TokenUsage, TranslateError,
};
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Subscription-backed translation through Codex CLI (0.154.0 or later).
/// Auth stays in CODEX_HOME; model tools cannot access the filesystem.
pub struct CodexProvider {
    model: Option<String>,
    reasoning_effort: Option<String>,
    system_prompt: String,
}

impl CodexProvider {
    pub fn new(
        model: Option<String>,
        reasoning_effort: Option<String>,
        system_prompt: String,
    ) -> Self {
        Self {
            model,
            reasoning_effort,
            system_prompt,
        }
    }

    fn command(&self, directory: &Path, instructions: &Path) -> Command {
        let mut command = Command::new("codex");
        command
            .args([
                "exec",
                "--json",
                "--ephemeral",
                "--ignore-user-config",
                "--ignore-rules",
                "--strict-config",
                "--skip-git-repo-check",
                "--color",
                "never",
            ])
            .current_dir(directory);
        for setting in [
            "model_provider=\"openai\"",
            "forced_login_method=\"chatgpt\"",
            "history.persistence=\"none\"",
            "approval_policy=\"never\"",
            "default_permissions=\"yeokja\"",
            "permissions.yeokja.filesystem={\"/\"=\"deny\"}",
            "permissions.yeokja.network.enabled=false",
            "project_doc_max_bytes=0",
            "web_search=\"disabled\"",
            "mcp_servers={}",
        ] {
            command.args(["-c", setting]);
        }
        // No shell, file viewer, computer/browser, plugins, hooks or delegated agents.
        // The deny-all permission profile is also enforced for sandboxed tool calls.
        for feature in [
            "shell_tool",
            "unified_exec",
            "view_image",
            "apps",
            "browser_use",
            "computer_use",
            "code_mode",
            "code_mode_host",
            "multi_agent",
            "plugins",
            "remote_plugin",
            "hooks",
            "image_generation",
            "shell_snapshot",
            "skill_search",
            "skill_mcp_dependency_install",
        ] {
            command.args(["--disable", feature]);
        }
        command.arg("-c").arg(config_string(
            "model_instructions_file",
            &instructions.to_string_lossy(),
        ));
        if let Some(model) = &self.model {
            command.args(["--model", model]);
        }
        if let Some(effort) = &self.reasoning_effort {
            command
                .arg("-c")
                .arg(config_string("model_reasoning_effort", effort));
        }
        command
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY")
            .env_remove("OPENAI_BASE_URL")
            .kill_on_drop(true);
        command
    }
}

fn config_string(key: &str, value: &str) -> String {
    format!("{key}={}", toml::Value::String(value.to_string()))
}

fn cli_error(message: impl std::fmt::Display) -> TranslateError {
    TranslateError::Api {
        status: 0,
        message: format!("Codex CLI: {message}"),
    }
}

#[async_trait]
impl LlmProvider for CodexProvider {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, TranslateError> {
        // Never run in the caller's project: avoid project config, instructions and discovery.
        let directory = tempfile::Builder::new()
            .prefix("yeokja-codex-")
            .tempdir()
            .map_err(cli_error)?;
        let instructions = directory.path().join("instructions.md");
        tokio::fs::write(&instructions, &self.system_prompt)
            .await
            .map_err(cli_error)?;
        let command = self.command(directory.path(), &instructions);
        run_command(command, &request.prompt).await
    }
}

async fn run_command(
    mut command: Command,
    prompt: &str,
) -> Result<CompletionResponse, TranslateError> {
    let mut child = command.spawn().map_err(|e| {
        cli_error(format!(
            "failed to start (install Codex CLI >= 0.154.0): {e}"
        ))
    })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| cli_error("stdin unavailable"))?;
    // Drain both output pipes while sending large prompts to avoid pipe deadlocks.
    let operation = async {
        let write = async {
            stdin.write_all(prompt.as_bytes()).await?;
            stdin.shutdown().await?;
            drop(stdin);
            Ok::<_, std::io::Error>(())
        };
        let (write_result, output) = tokio::join!(write, child.wait_with_output());
        let output = output.map_err(cli_error)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = if stderr.trim().is_empty() {
                &stdout
            } else {
                &stderr
            };
            return Err(cli_error(format!(
                "exited with {}: {detail}",
                output.status
            )));
        }
        write_result.map_err(cli_error)?;
        parse_response(&output.stdout)
    };
    tokio::time::timeout(Duration::from_secs(600), operation)
        .await
        .map_err(|_| cli_error("timed out after 600 seconds"))?
}

fn parse_response(stdout: &[u8]) -> Result<CompletionResponse, TranslateError> {
    let stdout = std::str::from_utf8(stdout).map_err(|e| TranslateError::Parse(e.to_string()))?;
    let mut text = None;
    let mut usage = None;
    let mut completed = false;
    let mut last_error = None;
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let event: Value = serde_json::from_str(line)
            .map_err(|e| TranslateError::Parse(format!("Invalid Codex JSONL: {e}")))?;
        match event["type"].as_str() {
            Some("item.completed") if event["item"]["type"] == "agent_message" => {
                let item = &event["item"];
                if item["phase"] != "commentary" {
                    text = item["text"].as_str().map(str::to_owned);
                }
            }
            Some("turn.completed") => {
                completed = true;
                if !event["usage"].is_null() {
                    usage = Some(
                        serde_json::from_value::<TokenUsage>(event["usage"].clone()).map_err(
                            |e| TranslateError::Parse(format!("Invalid Codex usage: {e}")),
                        )?,
                    );
                }
            }
            Some("turn.failed") => {
                return Err(cli_error(
                    event["error"]["message"].as_str().unwrap_or("turn failed"),
                ));
            }
            Some("error") => last_error = event["message"].as_str().map(str::to_owned),
            _ => {}
        }
    }
    if !completed {
        return Err(cli_error(
            last_error
                .as_deref()
                .unwrap_or("stream ended without turn.completed"),
        ));
    }
    let text = text
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| TranslateError::Parse("Codex returned no final assistant text".into()))?;
    Ok(CompletionResponse { text, usage })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUCCESS: &str = r#"{"type":"thread.started","thread_id":"example"}
{"type":"item.completed","item":{"id":"c","type":"agent_message","phase":"commentary","text":"Working..."}}
{"type":"item.completed","item":{"id":"a","type":"agent_message","phase":"final_answer","text":"[1] 안녕하세요."}}
{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":20}}
"#;

    #[test]
    fn returns_final_translation_and_usage() {
        let response = parse_response(SUCCESS.as_bytes()).unwrap();
        assert_eq!(response.text, "[1] 안녕하세요.");
        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 20);
    }

    #[test]
    fn rejects_failed_incomplete_and_malformed_streams() {
        for stream in [
            "",
            "not json",
            r#"{"type":"turn.completed"}"#,
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"partial"}}"#,
            r#"{"type":"turn.failed","error":{"message":"quota exceeded"}}"#,
        ] {
            assert!(parse_response(stream.as_bytes()).is_err(), "{stream}");
        }
    }

    #[test]
    fn reports_terminal_failure_after_partial_text() {
        let stream = SUCCESS.replace(r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":20}}"#,
            r#"{"type":"turn.failed","error":{"message":"quota exceeded"}}"#);
        let error = parse_response(stream.as_bytes()).unwrap_err();
        assert!(error.to_string().contains("quota exceeded"));
    }

    #[test]
    fn tolerates_reconnect_events_and_older_messages_without_phase() {
        let stream = r#"{"type":"error","message":"Reconnecting..."}
{"type":"item.completed","item":{"type":"agent_message","text":"translation"}}
{"type":"turn.completed"}"#;
        assert_eq!(
            parse_response(stream.as_bytes()).unwrap().text,
            "translation"
        );
    }
    #[test]
    fn command_enforces_isolation_and_passes_model_effort_and_instruction_path() {
        let provider = CodexProvider::new(
            Some("chosen-model".into()),
            Some("high".into()),
            "translate".into(),
        );
        let command = provider.command(
            Path::new("/tmp/empty"),
            Path::new("/tmp/a \"quoted\" path/instructions.md"),
        );
        let args: Vec<_> = command
            .as_std()
            .get_args()
            .map(|a| a.to_str().unwrap())
            .collect();
        for flag in [
            "--ephemeral",
            "--ignore-user-config",
            "--ignore-rules",
            "--strict-config",
            "--skip-git-repo-check",
            "--json",
        ] {
            assert!(args.contains(&flag), "missing {flag}");
        }
        assert!(args.windows(2).any(|a| a == ["--model", "chosen-model"]));
        let mut settings = toml::Table::new();
        for pair in args.windows(2).filter(|a| a[0] == "-c") {
            let table: toml::Table = pair[1].parse().unwrap();
            settings.extend(table);
        }
        assert_eq!(settings["model_reasoning_effort"].as_str(), Some("high"));
        assert_eq!(
            settings["model_instructions_file"].as_str(),
            Some("/tmp/a \"quoted\" path/instructions.md")
        );
        assert_eq!(settings["default_permissions"].as_str(), Some("yeokja"));
        // Parse each nested override separately: a TOML table merge here would overwrite siblings.
        assert!(args.contains(&"permissions.yeokja.filesystem={\"/\"=\"deny\"}"));
        assert!(args.contains(&"permissions.yeokja.network.enabled=false"));
        assert_eq!(settings["approval_policy"].as_str(), Some("never"));
        assert_eq!(settings["forced_login_method"].as_str(), Some("chatgpt"));
        assert_eq!(settings["history"]["persistence"].as_str(), Some("none"));
        assert_eq!(settings["project_doc_max_bytes"].as_integer(), Some(0));
        for feature in [
            "shell_tool",
            "view_image",
            "apps",
            "browser_use",
            "computer_use",
            "plugins",
            "hooks",
            "multi_agent",
            "code_mode",
            "code_mode_host",
        ] {
            assert!(
                args.windows(2).any(|a| a == ["--disable", feature]),
                "{feature}"
            );
        }
        for key in ["OPENAI_API_KEY", "CODEX_API_KEY", "OPENAI_BASE_URL"] {
            assert!(
                command
                    .as_std()
                    .get_envs()
                    .any(|(name, value)| name == key && value.is_none())
            );
        }
        assert_eq!(args.last(), Some(&"-"));
        assert_eq!(
            command.as_std().get_current_dir(),
            Some(Path::new("/tmp/empty"))
        );
    }

    #[test]
    fn omitted_model_and_effort_use_codex_defaults() {
        let provider = CodexProvider::new(None, None, "translate".into());
        let command = provider.command(Path::new("/tmp"), Path::new("/tmp/instructions.md"));
        let args: Vec<_> = command
            .as_std()
            .get_args()
            .map(|a| a.to_str().unwrap())
            .collect();
        assert!(!args.contains(&"--model"));
        assert!(
            !args
                .iter()
                .any(|a| a.starts_with("model_reasoning_effort="))
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn subprocess_receives_prompt_on_stdin_and_returns_translation() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", r#"read -r prompt
[ "$prompt" = 'Translate "hello" $(literal)' ] || exit 42
printf '%s\n' '{"type":"item.completed","item":{"type":"agent_message","text":"안녕하세요"}}' '{"type":"turn.completed"}'"#])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        let response = run_command(command, "Translate \"hello\" $(literal)\n")
            .await
            .unwrap();
        assert_eq!(response.text, "안녕하세요");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn subprocess_failure_reports_stderr_even_with_partial_output() {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "cat >/dev/null; printf 'login required' >&2; exit 7"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let error = run_command(command, "prompt").await.unwrap_err();
        assert!(error.to_string().contains("login required"));
        assert!(error.to_string().contains('7'));
    }
    /// No subscription/model call: the real CLI talks only to a loopback Responses fixture.
    #[cfg(target_os = "macos")]
    #[tokio::test]
    #[ignore = "requires Codex CLI >= 0.154.0 and localhost socket/sandbox permissions"]
    async fn real_cli_enforces_file_denial_and_ephemeral_instructions() {
        use std::io::{BufRead, BufReader, Read, Write};
        use std::net::TcpListener;

        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let work = root.path().join("work");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&work).unwrap();
        let victim = root.path().join("secret.txt");
        std::fs::write(&victim, "PRIVATE_SENTINEL\n").unwrap();
        let instructions = root.path().join("instructions.md");
        std::fs::write(&instructions, "Only translate the input.").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let victim_path = victim.clone();
        let server = std::thread::spawn(move || {
            let mut requests = Vec::new();
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            while requests.len() < 2 {
                assert!(
                    std::time::Instant::now() < deadline,
                    "CLI did not send requests"
                );
                let (mut socket, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => panic!("{e}"),
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                let mut reader = BufReader::new(socket.try_clone().unwrap());
                let mut length = None;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = Some(value.trim().parse::<usize>().unwrap());
                    }
                }
                let mut body = vec![0; length.unwrap()];
                reader.read_exact(&mut body).unwrap();
                requests.push(serde_json::from_slice::<Value>(&body).unwrap());
                let item = if requests.len() == 1 {
                    serde_json::json!({"id":"call_test","call_id":"call_test","type":"custom_tool_call","name":"apply_patch","input":format!("*** Begin Patch\n*** Update File: {}\n@@\n-PRIVATE_SENTINEL\n+CHANGED\n*** End Patch", victim_path.display())})
                } else {
                    serde_json::json!({"id":"msg_test","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"[1] 안녕하세요.","annotations":[]}]})
                };
                let events = [
                    serde_json::json!({"type":"response.created","response":{"id":"resp_test","object":"response","status":"in_progress","output":[]}}),
                    serde_json::json!({"type":"response.output_item.done","output_index":0,"item":item}),
                    serde_json::json!({"type":"response.completed","response":{"id":"resp_test","status":"completed","output":[item],"usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}}),
                ];
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n").unwrap();
                for event in events {
                    write!(socket, "data: {event}\n\n").unwrap();
                }
            }
            requests
        });
        let provider =
            CodexProvider::new(Some("gpt-5.4".into()), Some("high".into()), "unused".into());
        let mut command = provider.command(&work, &instructions);
        // Override only transport/auth for this test; production uses ChatGPT subscription auth.
        command.env("CODEX_HOME", &home)
            .args(["-c", "model_provider=\"fixture\""])
            .arg("-c").arg(format!("model_providers.fixture={{name=\"Fixture\",base_url=\"http://{address}/v1\",wire_api=\"responses\",requires_openai_auth=false}}"));
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            run_command(command, "Translate hello"),
        )
        .await;
        let requests = server.join().unwrap();
        assert_eq!(result.unwrap().unwrap().text, "[1] 안녕하세요.");
        assert_eq!(requests[0]["model"], "gpt-5.4");
        assert_eq!(requests[0]["reasoning"]["effort"], "high");
        assert_eq!(requests[0]["instructions"], "Only translate the input.");
        assert_eq!(
            std::fs::read_to_string(victim).unwrap(),
            "PRIVATE_SENTINEL\n"
        );
        let denial = requests[1]["input"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["type"] == "custom_tool_call_output")
            .unwrap();
        assert!(denial["output"].as_str().unwrap().contains("failed"));
        assert!(!home.join("sessions").exists());
        assert!(!home.join("history.jsonl").exists());
    }
}
