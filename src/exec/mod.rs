use async_trait::async_trait;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait]
pub trait Executor: Send + Sync {
    async fn run(&self, spec: &CommandSpec) -> anyhow::Result<RunOutput>;
}

#[derive(Default)]
pub struct TokioExecutor;

impl TokioExecutor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Executor for TokioExecutor {
    async fn run(&self, spec: &CommandSpec) -> anyhow::Result<RunOutput> {
        let mut cmd = tokio::process::Command::new(&spec.program);
        cmd.args(&spec.args);
        if let Some(cwd) = &spec.cwd {
            cmd.current_dir(cwd);
        }
        let output = cmd.output().await?;
        Ok(RunOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tokio_executor_runs_echo() -> anyhow::Result<()> {
        let exec = TokioExecutor::new();
        let out = exec
            .run(&CommandSpec {
                program: "sh".into(),
                args: vec!["-c".into(), "echo hello".into()],
                cwd: None,
            })
            .await?;
        assert_eq!(out.status, 0);
        assert_eq!(out.stdout, "hello\n");
        Ok(())
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    /// Records every CommandSpec it receives and returns programmed outputs FIFO.
    /// Used by engine tests to assert command shape without spawning processes.
    pub struct MockExecutor {
        pub calls: Mutex<Vec<CommandSpec>>,
        outputs: Mutex<VecDeque<RunOutput>>,
    }

    impl MockExecutor {
        pub fn with_outputs(outputs: Vec<RunOutput>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                outputs: Mutex::new(outputs.into()),
            }
        }
    }

    #[async_trait]
    impl Executor for MockExecutor {
        async fn run(&self, spec: &CommandSpec) -> anyhow::Result<RunOutput> {
            self.calls.lock().unwrap().push(spec.clone());
            self.outputs
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| anyhow::anyhow!("MockExecutor: no programmed output left"))
        }
    }
}
