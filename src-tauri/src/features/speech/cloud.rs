use std::path::Path;

/// Transcribe audio via cloud API (Groq or Deepgram).
pub fn cloud_transcribe(
    wav_path: &str,
    provider: &str,
    api_key: &str,
    language: &str,
) -> Result<String, String> {
    if !Path::new(wav_path).is_file() {
        return Err(format!("WAV file not found: {}", wav_path));
    }
    if api_key.is_empty() {
        return Err("API key required for cloud transcription".to_string());
    }

    let text = match provider {
        "groq" => cloud_groq(wav_path, api_key, language)?,
        "deepgram" => cloud_deepgram(wav_path, api_key, language)?,
        _ => return Err(format!("Unknown cloud provider: {}", provider)),
    };

    Ok(text.trim().to_string())
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))
}

/// Transcribe via Groq Whisper API (OpenAI-compatible multipart).
fn cloud_groq(wav_path: &str, api_key: &str, language: &str) -> Result<String, String> {
    let file_bytes = std::fs::read(wav_path)
        .map_err(|e| format!("Failed to read WAV file: {}", e))?;

    let file_part = reqwest::blocking::multipart::Part::bytes(file_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Multipart error: {}", e))?;

    let mut form = reqwest::blocking::multipart::Form::new()
        .part("file", file_part)
        .text("model", "whisper-large-v3")
        .text("response_format", "json");

    if !language.is_empty() && language != "auto" {
        form = form.text("language", language.to_string());
    }

    let resp = client()?
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .map_err(|e| format!("Groq request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Groq error: HTTP {}", resp.status()));
    }

    let data: serde_json::Value = resp.json().map_err(|e| format!("Groq response parse error: {}", e))?;
    Ok(data
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string())
}

/// Transcribe via Deepgram API (raw WAV body POST).
fn cloud_deepgram(wav_path: &str, api_key: &str, language: &str) -> Result<String, String> {
    let file_bytes = std::fs::read(wav_path)
        .map_err(|e| format!("Failed to read WAV file: {}", e))?;

    let lang = if language.is_empty() || language == "auto" {
        "en"
    } else {
        language
    };

    let url = format!(
        "https://api.deepgram.com/v1/listen?model=nova-2&language={}&smart_format=true",
        lang
    );

    let resp = client()?
        .post(&url)
        .header("Authorization", format!("Token {}", api_key))
        .header("Content-Type", "audio/wav")
        .body(file_bytes)
        .send()
        .map_err(|e| format!("Deepgram request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Deepgram error: HTTP {}", resp.status()));
    }

    let data: serde_json::Value = resp.json().map_err(|e| format!("Deepgram response parse error: {}", e))?;

    // Extract transcript from Deepgram response structure
    Ok(data["results"]["channels"][0]["alternatives"][0]["transcript"]
        .as_str()
        .unwrap_or("")
        .to_string())
}
