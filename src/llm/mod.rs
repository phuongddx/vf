use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub system: Option<String>,
    pub user: String,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub text: String,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse>;
}

pub mod anthropic;
pub mod openai;
pub mod registry;

#[cfg(test)]
mod tests {
    use super::*;

    struct UpperEcho;

    #[async_trait]
    impl LlmClient for UpperEcho {
        async fn complete(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
            Ok(ChatResponse { text: req.user.to_uppercase() })
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
}
