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

pub async fn run() -> anyhow::Result<()> {
    let exec = crate::exec::TokioExecutor::new();
    for tool in TOOLS {
        println!("{}", check_tool(&exec, tool).await);
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
}
