use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::Profile;

/// Request structure for OpenAI-compatible chat completions API
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Response structure for non-streaming chat completions
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    _usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageResponse,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    _prompt_tokens: u32,
    _completion_tokens: u32,
    _total_tokens: u32,
}

/// AI Provider client that works with OpenAI-compatible APIs
pub struct AIClient {
    client: Client,
    profile: Profile,
}

impl AIClient {
    /// Create a new AI client from a profile
    pub fn new(profile: &Profile) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(profile.timeout))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            profile: profile.clone(),
        })
    }

    /// Send a chat request and return the full response
    pub async fn chat(&self, system_prompt: &str, user_message: &str) -> Result<String> {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            },
        ];

        self.send_messages(messages).await
    }

    /// Send a chat request with conversation history
    pub async fn chat_with_history(
        &self,
        system_prompt: &str,
        history: &[(String, String)],
        user_message: &str,
    ) -> Result<String> {
        let mut messages = Vec::with_capacity(history.len() + 2);
        messages.push(Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        });

        for (role, content) in history {
            messages.push(Message {
                role: role.clone(),
                content: content.clone(),
            });
        }

        messages.push(Message {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        self.send_messages(messages).await
    }

    /// Send a streaming chat request and emit content chunks as they arrive.
    pub async fn chat_stream<F>(
        &self,
        system_prompt: &str,
        user_message: &str,
        mut on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(&str) -> Result<()>,
    {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            },
        ];

        self.stream_messages(messages, &mut on_chunk).await
    }

    /// Send a streaming chat request with conversation history.
    pub async fn chat_stream_with_history<F>(
        &self,
        system_prompt: &str,
        history: &[(String, String)],
        user_message: &str,
        mut on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(&str) -> Result<()>,
    {
        let mut messages = Vec::with_capacity(history.len() + 2);
        messages.push(Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        });

        for (role, content) in history {
            messages.push(Message {
                role: role.clone(),
                content: content.clone(),
            });
        }

        messages.push(Message {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        self.stream_messages(messages, &mut on_chunk).await
    }

    async fn stream_messages<F>(&self, messages: Vec<Message>, on_chunk: &mut F) -> Result<String>
    where
        F: FnMut(&str) -> Result<()>,
    {
        let request = self.build_request_from_messages(messages, true)?;
        let response = request
            .send()
            .await
            .context("Failed to send request to AI API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("API request failed with status {}: {}", status, error_text);
        }

        let mut stream = response.bytes_stream();
        let mut pending = String::new();
        let mut full_text = String::new();

        while let Some(item) = stream.next().await {
            let chunk = item.context("Failed to read stream chunk")?;
            let text = String::from_utf8_lossy(&chunk);
            pending.push_str(&text);

            while let Some(newline_pos) = pending.find('\n') {
                let line = pending[..newline_pos].trim_end_matches('\r').to_string();
                pending.drain(..=newline_pos);
                let compact = line.trim();

                if compact.is_empty() {
                    continue;
                }

                if let Some(data) = compact.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }

                    if let Some(content) = extract_stream_delta_content(data) {
                        if !content.is_empty() {
                            full_text.push_str(&content);
                            on_chunk(&content)?;
                        }
                    }
                }
            }
        }

        // Handle a trailing line without newline.
        let compact = pending.trim();
        if let Some(data) = compact.strip_prefix("data: ") {
            if data != "[DONE]" {
                if let Some(content) = extract_stream_delta_content(data) {
                    if !content.is_empty() {
                        full_text.push_str(&content);
                        on_chunk(&content)?;
                    }
                }
            }
        }

        Ok(full_text)
    }

    async fn send_messages(&self, messages: Vec<Message>) -> Result<String> {
        let request = self.build_request_from_messages(messages, false)?;

        let response = request
            .send()
            .await
            .context("Failed to send request to AI API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("API request failed with status {}: {}", status, error_text);
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse API response")?;

        chat_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from AI"))
    }

    /// Build the HTTP request for chat completion
    fn build_request(&self, system_prompt: &str, user_message: &str) -> Result<RequestBuilder> {
        self.build_request_from_messages(
            vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            false,
        )
    }

    fn build_request_from_messages(
        &self,
        messages: Vec<Message>,
        stream: bool,
    ) -> Result<RequestBuilder> {
        let mut url = self.profile.base_url.trim_end_matches('/').to_string();
        url.push_str("/chat/completions");

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.profile.api_key)
                .parse()
                .context("Invalid API key format")?,
        );

        let request = ChatRequest {
            model: self.profile.model.clone(),
            messages,
            stream,
            max_tokens: if self.profile.max_tokens > 0 {
                Some(self.profile.max_tokens)
            } else {
                None
            },
            temperature: if self.profile.temperature >= 0.0 {
                Some(self.profile.temperature)
            } else {
                None
            },
        };

        Ok(self.client.post(&url).headers(headers).json(&request))
    }

    /// Test the connection to the API
    pub async fn test_connection(&self) -> Result<()> {
        let test_request = self.build_request(
            "You are a test assistant.",
            "Respond with only the word 'OK'.",
        )?;

        let response = test_request
            .send()
            .await
            .context("Failed to connect to API")?;

        if response.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("API connection failed with status: {}", response.status())
        }
    }
}

fn extract_stream_delta_content(data: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(data).ok()?;

    // OpenAI-compatible stream format.
    if let Some(content) = value
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("delta"))
        .and_then(|delta| delta.get("content"))
        .and_then(|content| content.as_str())
    {
        return Some(content.to_string());
    }

    // Some providers may stream `text` directly.
    if let Some(content) = value
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("text"))
        .and_then(|content| content.as_str())
    {
        return Some(content.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_request() {
        use crate::config::Profile;

        let profile = Profile::new("test-key".to_string());
        let client = AIClient::new(&profile).unwrap();

        let request = client.build_request("System prompt", "User message");
        assert!(request.is_ok());
    }

    #[test]
    fn test_extract_stream_delta_content() {
        let data = r#"{"choices":[{"delta":{"content":"hello"}}]}"#;
        assert_eq!(extract_stream_delta_content(data).as_deref(), Some("hello"));
    }
}
