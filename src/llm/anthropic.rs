use super::{ChatRequest, ChatResponse, LlmClient};
use async_trait::async_trait;

pub struct AnthropicClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl AnthropicClient {
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
impl LlmClient for AnthropicClient {
    async fn complete(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": [{ "role": "user", "content": req.user }],
        });
        if let Some(s) = &req.system {
            body["system"] = serde_json::json!(s);
        }
        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let val: serde_json::Value = resp.json().await?;
        let text = val["content"]
            .as_array()
            .and_then(|parts| {
                parts
                    .iter()
                    .find(|p| p["type"] == "text")
                    .and_then(|p| p["text"].as_str())
            })
            .unwrap_or_default()
            .to_string();
        Ok(ChatResponse { text })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{ChatRequest, LlmClient};
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn completes_against_mock() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": [{ "type": "text", "text": "bonjour" }]
            })))
            .mount(&server)
            .await;

        let client = AnthropicClient::new(server.uri(), "test-key".into(), "claude-test".into());
        let resp = client
            .complete(&ChatRequest { system: Some("sois bref".into()), user: "salut".into() })
            .await?;
        assert_eq!(resp.text, "bonjour");
        Ok(())
    }
}
