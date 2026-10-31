use super::{ChatRequest, ChatResponse, LlmClient};
use async_trait::async_trait;

pub struct OpenAiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiClient {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            api_key,
            model,
        }
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn complete(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let mut messages = Vec::new();
        if let Some(s) = &req.system {
            messages.push(serde_json::json!({ "role": "system", "content": s }));
        }
        messages.push(serde_json::json!({ "role": "user", "content": req.user }));

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({ "model": self.model, "messages": messages }))
            .send()
            .await?
            .error_for_status()?;
        let body: serde_json::Value = resp.json().await?;
        let text = body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        Ok(ChatResponse { text })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{ChatRequest, LlmClient};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn completes_against_mock() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{ "message": { "role": "assistant", "content": "hello back" } }]
            })))
            .mount(&server)
            .await;

        let client = OpenAiClient::new(
            format!("{}/v1", server.uri()),
            "test-key".into(),
            "gpt-test".into(),
        );
        let resp = client
            .complete(&ChatRequest { system: Some("be brief".into()), user: "hi".into() })
            .await?;
        assert_eq!(resp.text, "hello back");
        Ok(())
    }
}
