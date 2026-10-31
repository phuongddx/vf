use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub system: Option<String>,
    pub user: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub text: String,
    pub usage: Option<Usage>,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse>;
}

pub mod anthropic;
pub mod openai;
pub mod registry;

/// Whether this binary supports HTTPS (cargo feature `tls`).
pub const TLS_ENABLED: bool = cfg!(feature = "tls");

/// Reject https base URLs when the binary lacks the `tls` feature.
pub fn check_tls(base_url: &str) -> anyhow::Result<()> {
    if base_url.starts_with("https://") && !TLS_ENABLED {
        anyhow::bail!(
            "binary built without TLS cannot use https endpoint '{base_url}' — \
             use a release binary (github.com/phuongddx/vf/releases) \
             or rebuild with: cargo build --release --features tls"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UpperEcho;

    #[async_trait]
    impl LlmClient for UpperEcho {
        async fn complete(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
            Ok(ChatResponse { text: req.user.to_uppercase(), usage: None })
        }
    }

    #[tokio::test]
    async fn trait_object_works() -> anyhow::Result<()> {
        let c: Box<dyn LlmClient> = Box::new(UpperEcho);
        let r = c
            .complete(&ChatRequest { system: None, user: "hi".into() })
            .await?;
        assert_eq!(r.text, "HI");
        Ok(())
    }

    #[test]
    fn tls_guard_classifies_urls() {
        assert!(super::check_tls("http://localhost:9/x").is_ok());
        #[cfg(not(feature = "tls"))]
        {
            let err = super::check_tls("https://api.z.ai/api/anthropic")
                .err()
                .unwrap();
            assert!(err.to_string().contains("without TLS"));
        }
        #[cfg(feature = "tls")]
        assert!(super::check_tls("https://api.z.ai/api/anthropic").is_ok());
    }
}
