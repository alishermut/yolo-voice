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

    // Build system prompt with tone + terminology context
    let mut system_prompt = profile.system_prompt.clone();
    if profile.tone != "neutral" && !profile.tone.is_empty() {
        system_prompt.push_str(&format!("\n\nUse a {} tone.", profile.tone));
    }
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
        "groq" => call_openai(
            text,
            &system_prompt,
            if model.is_empty() {
                "llama-3.3-70b-versatile"
            } else {
                model
            },
            api_key,
            if base_url.is_empty() {
                "https://api.groq.com/openai"
            } else {
                base_url
            },
        ),
        _ => Err(format!("Unknown LLM provider: {}", provider)),
    }?;

    Ok(result)
}

/// Execute a voice command through an LLM provider.
/// Unlike `post_process_text`, this sends the raw transcript as a command
/// with a dedicated system prompt — no profiles, no terminology hints.
pub fn command_llm_call(
    transcript: &str,
    system_prompt: &str,
    provider: &str,
    model: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, String> {
    if transcript.trim().is_empty() {
        return Ok(String::new());
    }

    let result = match provider {
        "ollama" => call_ollama(
            transcript,
            system_prompt,
            if model.is_empty() { "llama3.1:8b" } else { model },
            if base_url.is_empty() {
                "http://localhost:11434"
            } else {
                base_url
            },
        ),
        "openai" => call_openai(
            transcript,
            system_prompt,
            if model.is_empty() { "gpt-4o-mini" } else { model },
            api_key,
            if base_url.is_empty() {
                "https://api.openai.com"
            } else {
                base_url
            },
        ),
        "claude" => call_claude(
            transcript,
            system_prompt,
            if model.is_empty() {
                "claude-sonnet-4-20250514"
            } else {
                model
            },
            api_key,
        ),
        "groq" => call_openai(
            transcript,
            system_prompt,
            if model.is_empty() {
                "llama-3.3-70b-versatile"
            } else {
                model
            },
            api_key,
            if base_url.is_empty() {
                "https://api.groq.com/openai"
            } else {
                base_url
            },
        ),
        _ => Err(format!("Unknown command LLM provider: {}", provider)),
    }?;

    Ok(result)
}

/// Classify whether a voice command needs screen context.
/// Returns true if the command references something visible on screen.
/// On error, defaults to false (text-only fallback).
pub fn classify_needs_vision(
    transcript: &str,
    provider: &str,
    model: &str,
    api_key: &str,
    base_url: &str,
) -> bool {
    const CLASSIFIER_PROMPT: &str = "\
You are a classifier. Given a voice command, respond with ONLY \"yes\" or \"no\".\n\
Respond \"yes\" if the command references something visible on screen, such as:\n\
- \"answer this message\", \"reply to this email\", \"summarize what's on screen\"\n\
- \"what does this say\", \"fix this error\", \"what app is open\"\n\
- \"read this\", \"describe what you see\", \"draft a response to this\"\n\
Respond \"no\" if the command is self-contained, such as:\n\
- \"write hello world in python\", \"draft an email to Bob\"\n\
- \"translate good morning to French\", \"write a haiku about clouds\"";

    let result = command_llm_call(
        transcript,
        CLASSIFIER_PROMPT,
        provider,
        model,
        api_key,
        base_url,
    );

    match result {
        Ok(response) => response.trim().to_lowercase().starts_with("yes"),
        Err(e) => {
            eprintln!("[vision] Intent classification failed, defaulting to text-only: {}", e);
            false
        }
    }
}

/// Result of detecting a vocabulary addition command from voice input.
#[derive(Debug, Clone)]
pub struct VocabCommand {
    pub term: String,
    pub full_form: Option<String>,
}

/// Detect if a voice command transcript is requesting to add a vocabulary term.
/// Returns Some(VocabCommand) if detected, None otherwise.
pub fn detect_vocab_command(transcript: &str, api_key: &str) -> Option<VocabCommand> {
    if api_key.trim().is_empty() {
        return None;
    }

    let prompt = format!(
        "Does the following text ask to add a vocabulary/dictionary term or substitution rule? \
         Text: \"{}\"\n\n\
         If YES, respond with JSON: {{\"add\": true, \"term\": \"THE_TERM\", \"full_form\": \"OPTIONAL_FULL_FORM_OR_NULL\"}}\n\
         If NO, respond with JSON: {{\"add\": false}}\n\n\
         Respond ONLY with valid JSON, nothing else.",
        transcript
    );

    let response = call_openai(
        &prompt,
        "You detect vocabulary addition requests. Output only valid JSON.",
        "openai/gpt-oss-120b",
        api_key,
        "https://api.groq.com/openai",
    )
    .ok()?;

    let trimmed = response.trim();
    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;

    if value.get("add")?.as_bool()? {
        let term = value.get("term")?.as_str()?.trim().to_string();
        if term.is_empty() {
            return None;
        }
        let full_form = value
            .get("full_form")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s.to_lowercase() != "null");
        Some(VocabCommand { term, full_form })
    } else {
        None
    }
}

/// Generate common misspelling/misrecognition variants for a term using an LLM.
/// Calls Groq (openai-compatible) to produce a JSON array of lowercase strings.
pub fn generate_misspelling_variants(term: &str, api_key: &str) -> Result<Vec<String>, String> {
    if term.trim().is_empty() {
        return Ok(Vec::new());
    }
    if api_key.trim().is_empty() {
        return Err("API key is required for generating vocab variants".to_string());
    }

    let prompt = format!(
        "List 3-5 common ways speech-to-text might misrecognize the word '{}'. \
         Return ONLY a JSON array of lowercase strings, nothing else.",
        term.trim()
    );

    let response = call_openai(
        &prompt,
        "You are a helpful assistant that outputs only valid JSON.",
        "openai/gpt-oss-120b",
        api_key,
        "https://api.groq.com/openai",
    )?;

    // Parse the JSON array from the response
    let trimmed = response.trim();
    let variants: Vec<String> = serde_json::from_str(trimmed)
        .map_err(|e| format!("Failed to parse LLM response as JSON array: {}. Response: {}", e, trimmed))?;

    Ok(variants)
}

fn call_ollama(text: &str, system_prompt: &str, model: &str, base_url: &str) -> Result<String, String> {
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let payload = serde_json::json!({
        "model": model,
        "prompt": text,
        "system": system_prompt,
        "stream": false,
    });

    let resp = super::http_client()
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

    let resp = super::http_client()
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

    let resp = super::http_client()
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
