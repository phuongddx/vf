use super::{ChatRequest, ChatResponse, LlmClient, Usage};
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
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
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
            .await
            .map_err(|e| anyhow::Error::new(crate::llm::LlmError::Network).context(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(anyhow::Error::new(crate::llm::LlmError::Http(status.as_u16())));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| anyhow::Error::new(crate::llm::LlmError::Network).context(e.to_string()))?;
        let text = body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let usage = body["usage"].as_object().map(|u| Usage {
            input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            output_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        });
        Ok(ChatResponse { text, usage })
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
                "choices": [{ "message": { "role": "assistant", "content": "hello back" } }],
                "usage": { "prompt_tokens": 7, "completion_tokens": 3 }
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
        let u = resp.usage.expect("usage parsed");
        assert_eq!((u.input_tokens, u.output_tokens), (7, 3));
        Ok(())
    }

    #[tokio::test]
    async fn http_500_maps_to_llm_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let client = OpenAiClient::new(format!("{}/v1", server.uri()), "k".into(), "m".into());
        let err = client
            .complete(&ChatRequest { system: None, user: "x".into() })
            .await
            .err()
            .unwrap();
        assert_eq!(
            err.downcast_ref::<crate::llm::LlmError>(),
            Some(&crate::llm::LlmError::Http(500)),
        );
    }
}
