use super::{ChatRequest, ChatResponse, LlmClient, LlmError};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub delays: Vec<std::time::Duration>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            delays: vec![
                std::time::Duration::from_secs(1),
                std::time::Duration::from_secs(3),
            ],
        }
    }
}

fn is_retryable(err: &anyhow::Error) -> bool {
    match err.downcast_ref::<LlmError>() {
        Some(LlmError::Http(429)) => true,
        Some(LlmError::Http(c)) if (500..600).contains(c) => true,
        Some(LlmError::Network) => true,
        _ => false,
    }
}

/// Delay for `attempt` (0-based): base delay ± 200 ms jitter, never negative.
fn jittered(delays: &[std::time::Duration], attempt: u32) -> std::time::Duration {
    let idx = attempt.min(delays.len().saturating_sub(1) as u32) as usize;
    let base_ms = delays[idx].as_millis() as i64;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let jitter = (nanos % 400) as i64 - 200;
    std::time::Duration::from_millis((base_ms + jitter).max(0) as u64)
}

pub struct RetryingClient {
    inner: Box<dyn LlmClient>,
    policy: RetryPolicy,
}

impl RetryingClient {
    pub fn new(inner: Box<dyn LlmClient>) -> Self {
        Self { inner, policy: RetryPolicy::default() }
    }

    pub fn with_policy(inner: Box<dyn LlmClient>, policy: RetryPolicy) -> Self {
        Self { inner, policy }
    }
}

#[async_trait]
impl LlmClient for RetryingClient {
    async fn complete(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let mut attempt = 0u32;
        loop {
            match self.inner.complete(req).await {
                Ok(resp) => return Ok(resp),
                Err(e) if is_retryable(&e) && attempt < self.policy.max_retries => {
                    tokio::time::sleep(jittered(&self.policy.delays, attempt)).await;
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{ChatRequest, ChatResponse, LlmClient, LlmError};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    struct Flaky {
        fails: u32,
        code: u16,
        calls: Arc<AtomicU32>,
    }

    #[async_trait]
    impl LlmClient for Flaky {
        async fn complete(&self, _req: &ChatRequest) -> anyhow::Result<ChatResponse> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n < self.fails {
                Err(anyhow::Error::new(if self.code == 0 {
                    LlmError::Network
                } else {
                    LlmError::Http(self.code)
                }))
            } else {
                Ok(ChatResponse { text: "ok".into(), usage: None })
            }
        }
    }

    fn fast() -> RetryPolicy {
        RetryPolicy { max_retries: 2, delays: vec![ms(1), ms(1)] }
    }

    fn ms(n: u64) -> std::time::Duration {
        std::time::Duration::from_millis(n)
    }

    #[tokio::test]
    async fn retries_5xx_then_succeeds() -> anyhow::Result<()> {
        let inner = Box::new(Flaky { fails: 2, code: 500, calls: Arc::new(AtomicU32::new(0)) });
        let c = RetryingClient::with_policy(inner, fast());
        let r = c.complete(&ChatRequest { system: None, user: "x".into() }).await?;
        assert_eq!(r.text, "ok");
        Ok(())
    }

    #[tokio::test]
    async fn exhausts_retries_on_persistent_500() {
        let inner = Box::new(Flaky { fails: 99, code: 500, calls: Arc::new(AtomicU32::new(0)) });
        let c = RetryingClient::with_policy(inner, fast());
        assert!(c.complete(&ChatRequest { system: None, user: "x".into() }).await.is_err());
    }

    #[tokio::test]
    async fn does_not_retry_400() {
        let calls = Arc::new(AtomicU32::new(0));
        let inner = Box::new(Flaky { fails: 99, code: 400, calls: calls.clone() });
        let c = RetryingClient::with_policy(inner, fast());
        assert!(c.complete(&ChatRequest { system: None, user: "x".into() }).await.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_network_errors() -> anyhow::Result<()> {
        let inner = Box::new(Flaky { fails: 1, code: 0, calls: Arc::new(AtomicU32::new(0)) });
        let c = RetryingClient::with_policy(inner, fast());
        assert!(c.complete(&ChatRequest { system: None, user: "x".into() }).await.is_ok());
        Ok(())
    }
}
