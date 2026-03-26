/// Multimodal vision API: send voice command text + screenshot to a cloud LLM.

use base64::Engine;

/// Send a voice command + screenshot to a vision-capable LLM.
/// `screenshot_bytes` is raw JPEG bytes (not base64).
pub fn vision_command(
    transcript: &str,
    screenshot_bytes: &[u8],
    system_prompt: &str,
    provider: &str,
    model: &str,
    api_key: &str,
) -> Result<String, String> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(screenshot_bytes);

    match provider {
        "openai" => call_openai_vision(
            transcript, &b64, system_prompt, model, api_key,
            "https://api.openai.com/v1/chat/completions",
        ),
        "groq" => call_openai_vision(
            transcript, &b64, system_prompt, model, api_key,
            "https://api.groq.com/openai/v1/chat/completions",
        ),
        "claude" => call_claude_vision(transcript, &b64, system_prompt, model, api_key),
        _ => Err(format!("Unsupported vision provider: {}", provider)),
    }
}

fn call_openai_vision(
    transcript: &str,
    screenshot_b64: &str,
    system_prompt: &str,
    model: &str,
    api_key: &str,
    endpoint: &str,
) -> Result<String, String> {
    let model = if model.is_empty() { "gpt-4o" } else { model };
    let data_url = format!("data:image/jpeg;base64,{}", screenshot_b64);

    let payload = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "messages": [
            {
                "role": "system",
                "content": system_prompt,
            },
            {
                "role": "user",
                "content": [
                    { "type": "text", "text": transcript },
                    {
                        "type": "image_url",
                        "image_url": { "url": data_url }
                    }
                ]
            }
        ]
    });

    let resp = super::http_client()
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .map_err(|e| format!("OpenAI vision request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("OpenAI vision error: HTTP {} — {}", status, body));
    }

    let data: serde_json::Value = resp
        .json()
        .map_err(|e| format!("OpenAI vision response parse error: {}", e))?;

    Ok(data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string())
}

fn call_claude_vision(
    transcript: &str,
    screenshot_b64: &str,
    system_prompt: &str,
    model: &str,
    api_key: &str,
) -> Result<String, String> {
    let model = if model.is_empty() {
        "claude-sonnet-4-20250514"
    } else {
        model
    };

    let payload = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/jpeg",
                            "data": screenshot_b64
                        }
                    },
                    {
                        "type": "text",
                        "text": transcript
                    }
                ]
            }
        ]
    });

    let resp = super::http_client()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .map_err(|e| format!("Claude vision request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("Claude vision error: HTTP {} — {}", status, body));
    }

    let data: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Claude vision response parse error: {}", e))?;

    let content = data
        .get("content")
        .and_then(|c| c.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    Ok(content.trim().to_string())
}
