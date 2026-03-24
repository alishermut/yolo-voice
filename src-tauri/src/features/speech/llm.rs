use super::Profile;

/// Post-process transcribed text through an LLM provider.
pub fn post_process_text(
    text: &str,
    profile: &Profile,
    provider: &str,
    model: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    // Build system prompt with terminology hint context
    let mut system_prompt = profile.system_prompt.clone();
    if !profile.terminology_hints.is_empty() {
        let dict_str = profile.terminology_hints.join(", ");
        system_prompt.push_str(&format!(
            "\n\nImportant terminology to preserve exactly: {}",
            dict_str
        ));
    }

    let result = match provider {
        "ollama" => call_ollama(
            text,
            &system_prompt,
            if model.is_empty() { "llama3.1:8b" } else { model },
            if base_url.is_empty() {
                "http://localhost:11434"
            } else {
                base_url
            },
        ),
        "openai" => call_openai(
            text,
            &system_prompt,
            if model.is_empty() { "gpt-4o-mini" } else { model },
            api_key,
            if base_url.is_empty() {
                "https://api.openai.com"
            } else {
                base_url
            },
        ),
        "claude" => call_claude(
            text,
            &system_prompt,
            if model.is_empty() {
                "claude-sonnet-4-20250514"
            } else {
                model
            },
            api_key,
        ),
        _ => Err(format!("Unknown LLM provider: {}", provider)),
    }?;

    Ok(result)
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))
}

fn call_ollama(text: &str, system_prompt: &str, model: &str, base_url: &str) -> Result<String, String> {
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let payload = serde_json::json!({
        "model": model,
        "prompt": text,
        "system": system_prompt,
        "stream": false,
    });

    let resp = client()?
        .post(&url)
        .json(&payload)
        .send()
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Ollama error: HTTP {}", resp.status()));
    }

    let data: serde_json::Value = resp.json().map_err(|e| format!("Ollama response parse error: {}", e))?;
    Ok(data
        .get("response")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .trim()
        .to_string())
}

fn call_openai(
    text: &str,
    system_prompt: &str,
    model: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, String> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let payload = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": text},
        ],
        "temperature": 0.3,
    });

    let resp = client()?
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .map_err(|e| format!("OpenAI request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("OpenAI error: HTTP {}", resp.status()));
    }

    let data: serde_json::Value = resp.json().map_err(|e| format!("OpenAI response parse error: {}", e))?;
    Ok(data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string())
}

fn call_claude(text: &str, system_prompt: &str, model: &str, api_key: &str) -> Result<String, String> {
    let payload = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [
            {"role": "user", "content": text},
        ],
    });

    let resp = client()?
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .map_err(|e| format!("Claude request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Claude error: HTTP {}", resp.status()));
    }

    let data: serde_json::Value = resp.json().map_err(|e| format!("Claude response parse error: {}", e))?;
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
