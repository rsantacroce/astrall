//! Provedores de LLM plugáveis: API da Anthropic e qualquer API compatível com a da OpenAI
//! (OpenAI, OpenRouter, Ollama, LM Studio…), com resposta em streaming (SSE).

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Anthropic,
    OpenaiCompat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub label: String,
    pub kind: Kind,
    pub base_url: String,
    pub model: String,
    pub needs_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSettings {
    pub active: String,
    pub providers: Vec<Provider>,
}

impl Default for LlmSettings {
    fn default() -> Self {
        let p = |id: &str, label: &str, kind, base: &str, model: &str, needs_key| Provider {
            id: id.into(),
            label: label.into(),
            kind,
            base_url: base.into(),
            model: model.into(),
            needs_key,
        };
        LlmSettings {
            // Local por padrão: a leitura não sai da máquina e não custa nada por uso.
            active: "ollama".into(),
            providers: vec![
                p(
                    "anthropic",
                    "Claude (Anthropic)",
                    Kind::Anthropic,
                    "https://api.anthropic.com",
                    "claude-opus-5-5",
                    true,
                ),
                p(
                    "openai",
                    "OpenAI",
                    Kind::OpenaiCompat,
                    "https://api.openai.com/v1",
                    "",
                    true,
                ),
                p(
                    "ollama",
                    "Ollama (local)",
                    Kind::OpenaiCompat,
                    "http://localhost:11434/v1",
                    "",
                    false,
                ),
                p(
                    "custom",
                    "Outro (compatível com OpenAI)",
                    Kind::OpenaiCompat,
                    "",
                    "",
                    true,
                ),
            ],
        }
    }
}

pub struct Outcome {
    pub text: String,
    /// A resposta foi cortada pelo limite de tokens.
    pub truncated: bool,
}

const ANTHROPIC_VERSION: &str = "2026-10-21";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

/// Modelos em que um bloqueio dos classificadores de segurança pode ser refeito, no servidor,
/// por outro modelo (`fallbacks: "default"`).
fn supports_fallbacks(model: &str) -> bool {
    [
        "claude-fable-5-2",
        "claude-fable-5-1",
        "claude-opus-5-5",
        "claude-opus-5",
        "claude-sonnet-5-5",
    ]
    .contains(&model)
}

fn base(p: &Provider) -> &str {
    p.base_url.trim().trim_end_matches('/')
}

fn request(
    client: &reqwest::Client,
    p: &Provider,
    key: Option<&str>,
    path: &str,
    body: Option<&Value>,
) -> reqwest::RequestBuilder {
    let url = match p.kind {
        Kind::Anthropic => format!("{}/v1/{path}", base(p)),
        Kind::OpenaiCompat => format!("{}/{path}", base(p)),
    };
    let mut req = if body.is_some() {
        client.post(url)
    } else {
        client.get(url)
    };
    match p.kind {
        Kind::Anthropic => {
            req = req.header("anthropic-version", ANTHROPIC_VERSION);
            if let Some(k) = key {
                req = req.header("x-api-key", k);
            }
            if body.is_some() && supports_fallbacks(&p.model) {
                req = req.header("anthropic-beta", FALLBACK_BETA);
            }
        }
        Kind::OpenaiCompat => {
            if let Some(k) = key {
                req = req.bearer_auth(k);
            }
        }
    }
    match body {
        Some(b) => req.json(b),
        None => req,
    }
}

async fn fail(resp: reqwest::Response) -> String {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    let detail = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| {
            v["error"]["message"]
                .as_str()
                .or(v["error"].as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| body.chars().take(300).collect());
    let lead = match status.as_u16() {
        401 | 403 => "Chave de API recusada pelo provedor",
        404 => "Endereço ou modelo não encontrado",
        429 => "Limite de uso do provedor atingido; tente de novo em instantes",
        500..=599 => "O provedor está com problemas no momento",
        _ => "O provedor recusou o pedido",
    };
    format!("{lead} (HTTP {}). {detail}", status.as_u16())
}

fn connect_error(p: &Provider, e: reqwest::Error) -> String {
    if e.is_connect() || e.is_timeout() {
        format!(
            "Não foi possível conectar a {}. Verifique a internet e o endereço do provedor.",
            base(p)
        )
    } else {
        format!("Falha de rede: {e}")
    }
}

pub async fn list_models(client: &reqwest::Client, p: &Provider, key: Option<&str>) -> Result<Vec<String>, String> {
    let resp = request(client, p, key, "models", None)
        .send()
        .await
        .map_err(|e| connect_error(p, e))?;
    if !resp.status().is_success() {
        return Err(fail(resp).await);
    }
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Resposta inesperada do provedor: {e}"))?;
    let mut ids: Vec<String> = v["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["id"].as_str().map(str::to_string))
        .collect();
    if p.kind == Kind::OpenaiCompat {
        ids.sort();
    }
    Ok(ids)
}

/// Envia o pedido e repassa o texto conforme chega. `cancel` interrompe a leitura.
pub async fn stream(
    client: &reqwest::Client,
    p: &Provider,
    key: Option<&str>,
    system: &str,
    user: &str,
    cancel: &AtomicBool,
    mut on_delta: impl FnMut(&str),
) -> Result<Outcome, String> {
    if p.model.trim().is_empty() {
        return Err("Escolha um modelo nas configurações do provedor.".into());
    }
    let (path, body) = match p.kind {
        Kind::Anthropic => {
            // Sem `thinking` nem `effort`: cada modelo usa o próprio padrão (raciocínio adaptativo).
            let mut b = json!({
                "model": p.model,
                "max_tokens": 64000,
                "stream": true,
                "system": system,
                "messages": [{ "role": "user", "content": user }],
            });
            if supports_fallbacks(&p.model) {
                b["fallbacks"] = json!("default");
            }
            ("messages", b)
        }
        Kind::OpenaiCompat => (
            "chat/completions",
            json!({
                "model": p.model,
                "stream": true,
                "messages": [{ "role": "system", "content": system }, { "role": "user", "content": user }],
            }),
        ),
    };
    let resp = request(client, p, key, path, Some(&body))
        .send()
        .await
        .map_err(|e| connect_error(p, e))?;
    if !resp.status().is_success() {
        return Err(fail(resp).await);
    }

    let mut text = String::new();
    let mut truncated = false;
    let mut buf: Vec<u8> = vec![];
    let mut bytes = resp.bytes_stream();
    while let Some(chunk) = bytes.next().await {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        buf.extend_from_slice(&chunk.map_err(|e| format!("A conexão caiu durante a resposta: {e}"))?);
        // Eventos SSE são linhas; só as "data:" importam aqui.
        while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line);
            let Some(data) = line.trim_end().strip_prefix("data:") else {
                continue;
            };
            let data = data.trim_start();
            if data == "[DONE]" {
                continue;
            }
            let Ok(ev) = serde_json::from_str::<Value>(data) else {
                continue;
            };
            match p.kind {
                Kind::Anthropic => match ev["type"].as_str() {
                    Some("content_block_delta") if ev["delta"]["type"] == "text_delta" => {
                        if let Some(t) = ev["delta"]["text"].as_str() {
                            text.push_str(t);
                            on_delta(t);
                        }
                    }
                    Some("message_delta") => match ev["delta"]["stop_reason"].as_str() {
                        Some("max_tokens") => truncated = true,
                        Some("refusal") => {
                            return Err("O modelo recusou este pedido. Tente outro modelo ou provedor.".into())
                        }
                        _ => {}
                    },
                    Some("error") => {
                        let msg = ev["error"]["message"].as_str().unwrap_or("erro desconhecido");
                        return Err(format!("O provedor interrompeu a resposta: {msg}"));
                    }
                    _ => {}
                },
                Kind::OpenaiCompat => {
                    if let Some(msg) = ev["error"]["message"].as_str() {
                        return Err(format!("O provedor interrompeu a resposta: {msg}"));
                    }
                    let choice = &ev["choices"][0];
                    if let Some(t) = choice["delta"]["content"].as_str() {
                        if !t.is_empty() {
                            text.push_str(t);
                            on_delta(t);
                        }
                    }
                    if choice["finish_reason"] == "length" {
                        truncated = true;
                    }
                }
            }
        }
    }
    if text.trim().is_empty() {
        return Err("O provedor não devolveu nenhum texto.".into());
    }
    Ok(Outcome { text, truncated })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallbacks_only_on_supported_models() {
        assert!(supports_fallbacks("claude-opus-5-5"));
        assert!(!supports_fallbacks("claude-haiku-5-5"));
        assert!(!supports_fallbacks("llama3"));
    }
}

#[cfg(test)]
mod stream_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Servidor de uma resposta só: devolve `body` como text/event-stream e o pedido recebido.
    async fn serve(body: &'static str, status: &'static str) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut req = vec![];
            let mut buf = [0u8; 4096];
            loop {
                let n = sock.read(&mut buf).await.unwrap();
                req.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&req);
                if let Some(head_end) = text.find("\r\n\r\n") {
                    let len = text
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if req.len() >= head_end + 4 + len {
                        break;
                    }
                }
            }
            let resp = format!(
                "HTTP/1.1 {status}\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            sock.write_all(resp.as_bytes()).await.unwrap();
            String::from_utf8_lossy(&req).to_string()
        });
        (format!("http://{addr}"), handle)
    }

    fn provider(kind: Kind, base: String, model: &str) -> Provider {
        Provider {
            id: "t".into(),
            label: "t".into(),
            kind,
            base_url: base,
            model: model.into(),
            needs_key: true,
        }
    }

    #[tokio::test]
    async fn anthropic_stream() {
        let body = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{}}\n\n\
event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"x\"}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"Olá, \"}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"Leão.\"}}\n\n\
event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n\
event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        let (base, server) = serve(body, "200 OK").await;
        let p = provider(Kind::Anthropic, base, "claude-opus-5-5");
        let mut deltas = vec![];
        let out = stream(
            &reqwest::Client::new(),
            &p,
            Some("k"),
            "sys",
            "user",
            &AtomicBool::new(false),
            |t| deltas.push(t.to_string()),
        )
        .await
        .unwrap();
        assert_eq!(out.text, "Olá, Leão.");
        assert_eq!(deltas.len(), 2);
        assert!(!out.truncated);
        let req = server.await.unwrap().to_lowercase();
        assert!(req.starts_with("post /v1/messages "));
        assert!(req.contains("x-api-key: k") && req.contains("anthropic-version: 2026-10-21"));
        assert!(
            req.contains("anthropic-beta: server-side-fallback-2026-07-01")
                && req.contains("\"fallbacks\":\"default\"")
        );
        assert!(!req.contains("temperature") && !req.contains("budget_tokens"));
    }

    #[tokio::test]
    async fn anthropic_refusal_and_http_error() {
        let body = "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"a\"}}\n\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"refusal\"}}\n\n";
        let (base, _s) = serve(body, "200 OK").await;
        let p = provider(Kind::Anthropic, base, "claude-haiku-5-5");
        let e = stream(
            &reqwest::Client::new(),
            &p,
            Some("k"),
            "s",
            "u",
            &AtomicBool::new(false),
            |_| {},
        )
        .await
        .err()
        .unwrap();
        assert!(e.contains("recusou"));

        let (base, _s) = serve(
            "{\"type\":\"error\",\"error\":{\"type\":\"authentication_error\",\"message\":\"invalid x-api-key\"}}",
            "401 Unauthorized",
        )
        .await;
        let p = provider(Kind::Anthropic, base, "claude-opus-5-5");
        let e = stream(
            &reqwest::Client::new(),
            &p,
            Some("k"),
            "s",
            "u",
            &AtomicBool::new(false),
            |_| {},
        )
        .await
        .err()
        .unwrap();
        assert!(
            e.contains("Chave de API recusada") && e.contains("invalid x-api-key"),
            "{e}"
        );
    }

    #[tokio::test]
    async fn openai_compatible_stream() {
        let body = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"}}]}\n\n\
data: {\"choices\":[{\"delta\":{\"content\":\"Sol \"}}]}\n\n\
data: {\"choices\":[{\"delta\":{\"content\":\"em Leão\"},\"finish_reason\":null}]}\n\n\
data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n\n\
data: [DONE]\n\n";
        let (base, server) = serve(body, "200 OK").await;
        let p = provider(Kind::OpenaiCompat, format!("{base}/v1/"), "llama3");
        let out = stream(
            &reqwest::Client::new(),
            &p,
            None,
            "sys",
            "user",
            &AtomicBool::new(false),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(out.text, "Sol em Leão");
        assert!(out.truncated);
        let req = server.await.unwrap().to_lowercase();
        assert!(req.starts_with("post /v1/chat/completions "));
        assert!(!req.contains("authorization:"));
    }
}
