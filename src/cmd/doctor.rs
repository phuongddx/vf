use crate::exec::{CommandSpec, Executor};

pub const TOOLS: &[&str] = &["git", "node", "npx", "ffmpeg"];

/// Check one tool: run `<tool> --version` and format a doctor line.
pub async fn check_tool(exec: &dyn Executor, tool: &str) -> String {
    let spec = CommandSpec {
        program: tool.to_string(),
        args: vec!["--version".into()],
        cwd: None,
    };
    match exec.run(&spec).await {
        Ok(out) if out.status == 0 => {
            let version = out.stdout.lines().next().unwrap_or("").trim();
            format!("doctor: {tool} OK ({version})")
        }
        _ => format!("doctor: {tool} MISSING"),
    }
}

pub async fn check_tls_status() -> String {
    if crate::llm::TLS_ENABLED {
        "doctor: tls OK (https supported)".into()
    } else {
        "doctor: tls MISSING (rebuild with --features tls for https)".into()
    }
}

pub async fn check_url(url: &str) -> String {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let start = std::time::Instant::now();
    match client.get(url).send().await {
        Ok(_) => format!("doctor: network {url} OK ({} ms)", start.elapsed().as_millis()),
        Err(_) => format!("doctor: network {url} UNREACHABLE"),
    }
}

pub async fn tool_available(exec: &dyn Executor, tool: &str) -> bool {
    let spec = CommandSpec {
        program: tool.to_string(),
        args: vec!["--version".into()],
        cwd: None,
    };
    matches!(exec.run(&spec).await, Ok(out) if out.status == 0)
}

pub async fn run() -> anyhow::Result<()> {
    let exec = crate::exec::TokioExecutor::new();
    for tool in TOOLS {
        println!("{}", check_tool(&exec, tool).await);
    }
    println!("{}", check_tls_status().await);
    let cfg = crate::config::Config::load()?.effective();
    for (_name, p) in &cfg.providers {
        let key_set = std::env::var(&p.api_key_env)
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        if !key_set {
            continue;
        }
        if p.base_url.starts_with("https://") && !crate::llm::TLS_ENABLED {
            println!("doctor: network {} SKIPPED (no tls in this binary)", p.base_url);
            continue;
        }
        println!("{}", check_url(&p.base_url).await);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::testing::MockExecutor;
    use crate::exec::RunOutput;

    fn ok_out(stdout: &str) -> RunOutput {
        RunOutput {
            status: 0,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    #[tokio::test]
    async fn check_tool_reports_ok_and_missing() -> anyhow::Result<()> {
        let mock = MockExecutor::with_outputs(vec![
            ok_out("git version 2.47.0\n"),
            RunOutput {
                status: 1,
                stdout: String::new(),
                stderr: "not found".into(),
            },
        ]);
        let ok = check_tool(&mock, "git").await;
        assert!(ok.contains("doctor: git OK"));
        assert!(ok.contains("2.47.0"));
        let missing = check_tool(&mock, "ffmpeg").await;
        assert_eq!(missing, "doctor: ffmpeg MISSING");
        let calls = mock.calls.lock().unwrap();
        assert_eq!(calls[0].args, vec!["--version".to_string()]);
        Ok(())
    }

    #[tokio::test]
    async fn tls_status_line_names_state() {
        let line = super::check_tls_status().await;
        assert!(line.starts_with("doctor: tls "));
        assert!(line.contains("tls OK (https supported)") || line.contains("tls MISSING"));
    }

    #[tokio::test]
    async fn check_url_reports_reachable_and_unreachable() -> anyhow::Result<()> {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let ok = super::check_url(&server.uri()).await;
        assert!(ok.contains("OK"), "line was: {ok}");
        let bad = super::check_url("http://127.0.0.1:9/").await;
        assert!(bad.contains("UNREACHABLE"), "line was: {bad}");
        Ok(())
    }

    #[tokio::test]
    async fn tool_available_reports_bool() {
        let mock = MockExecutor::with_outputs(vec![
            RunOutput { status: 0, stdout: "v".into(), stderr: String::new() },
            RunOutput { status: 127, stdout: String::new(), stderr: "not found".into() },
        ]);
        assert!(super::tool_available(&mock, "node").await);
        assert!(!super::tool_available(&mock, "ffmpeg").await);
    }
}
