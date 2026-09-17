use std::sync::Arc;

use reqwest::{
    Client, Response,
    header::{ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::RwLock;
use url::Url;

use crate::{
    error::{AnalyzerError, Result},
    models::{MatchMetadata, PlayerProfile, RateLimitState},
    rate_limit::RpmLimiter,
};

const API_ROOT: &str = "https://api.pubg.com/shards";
const MAX_TELEMETRY_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone)]
pub struct PubgClient {
    client: Client,
    api_key: Arc<str>,
    platform: Arc<str>,
    limiter: RpmLimiter,
    rate_limit: Arc<RwLock<RateLimitState>>,
}

impl PubgClient {
    pub fn new(api_key: impl Into<String>, platform: impl Into<String>, rpm: u32) -> Result<Self> {
        let api_key = api_key.into();
        let platform = platform.into();
        if api_key.trim().is_empty() {
            return Err(AnalyzerError::InvalidConfiguration(
                "PUBG API key is required".to_owned(),
            ));
        }
        if platform.is_empty()
            || !platform
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err(AnalyzerError::InvalidConfiguration(
                "invalid PUBG platform shard".to_owned(),
            ));
        }

        let client = Client::builder()
            .user_agent("PUBG-Analyzer/0.2")
            .gzip(true)
            .timeout(std::time::Duration::from_secs(45))
            .build()?;
        Ok(Self {
            client,
            api_key: Arc::from(api_key),
            platform: Arc::from(platform),
            limiter: RpmLimiter::new(rpm),
            rate_limit: Arc::new(RwLock::new(RateLimitState::default())),
        })
    }

    pub async fn lookup_player(&self, username: &str) -> Result<PlayerProfile> {
        self.limiter.acquire().await;
        let url = format!("{API_ROOT}/{}/players", self.platform);
        let response = self
            .authorized_get(&url)?
            .query(&[("filter[playerNames]", username)])
            .send()
            .await?;
        self.update_rate_limit(response.headers()).await;
        let document: PlayerDocument = self.checked_json(response).await?;
        let player = document.data.into_iter().next().ok_or_else(|| {
            AnalyzerError::InvalidConfiguration(format!(
                "player '{username}' was not found on {}",
                self.platform
            ))
        })?;
        let match_ids = player
            .relationships
            .matches
            .data
            .into_iter()
            .map(|item| item.id)
            .collect();
        Ok(PlayerProfile {
            account_id: player.id,
            name: player.attributes.name,
            platform: self.platform.to_string(),
            match_ids,
        })
    }

    pub async fn fetch_match(&self, match_id: &str) -> Result<MatchMetadata> {
        validate_identifier(match_id, "match ID")?;
        let url = format!("{API_ROOT}/{}/matches/{match_id}", self.platform);
        let response = self.public_get(&url).send().await?;
        let document: Value = self.checked_json(response).await?;
        parse_match_document(&document, &self.platform)
    }

    pub async fn fetch_telemetry(&self, telemetry_url: &str) -> Result<Vec<u8>> {
        let parsed = Url::parse(telemetry_url).map_err(|error| {
            AnalyzerError::InvalidTelemetry(format!("invalid telemetry URL: {error}"))
        })?;
        if parsed.scheme() != "https" || parsed.host_str() != Some("telemetry-cdn.pubg.com") {
            return Err(AnalyzerError::InvalidTelemetry(
                "telemetry URL must use the official PUBG telemetry CDN".to_owned(),
            ));
        }
        let response = self
            .public_get(parsed.as_str())
            .header(ACCEPT_ENCODING, "gzip")
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(status_error(response).await);
        }
        if response
            .content_length()
            .is_some_and(|size| size > MAX_TELEMETRY_BYTES)
        {
            return Err(AnalyzerError::InvalidTelemetry(
                "telemetry payload exceeds 128 MiB".to_owned(),
            ));
        }
        let bytes = response.bytes().await?;
        if bytes.len() as u64 > MAX_TELEMETRY_BYTES {
            return Err(AnalyzerError::InvalidTelemetry(
                "telemetry payload exceeds 128 MiB".to_owned(),
            ));
        }
        Ok(bytes.to_vec())
    }

    pub async fn rate_limit_state(&self) -> RateLimitState {
        self.rate_limit.read().await.clone()
    }

    fn authorized_get(&self, url: &str) -> Result<reqwest::RequestBuilder> {
        let authorization =
            HeaderValue::from_str(&format!("Bearer {}", self.api_key)).map_err(|_| {
                AnalyzerError::InvalidConfiguration("API key contains invalid bytes".to_owned())
            })?;
        Ok(self.public_get(url).header(AUTHORIZATION, authorization))
    }

    fn public_get(&self, url: &str) -> reqwest::RequestBuilder {
        self.client
            .get(url)
            .header(ACCEPT, "application/vnd.api+json")
            .header(USER_AGENT, "PUBG-Analyzer/0.2")
    }

    async fn checked_json<T: serde::de::DeserializeOwned>(&self, response: Response) -> Result<T> {
        if response.status().is_success() {
            return Ok(response.json().await?);
        }
        Err(status_error(response).await)
    }

    async fn update_rate_limit(&self, headers: &HeaderMap) {
        let parse_u32 = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse().ok())
        };
        let parse_i64 = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse().ok())
        };
        *self.rate_limit.write().await = RateLimitState {
            limit: parse_u32("x-ratelimit-limit"),
            remaining: parse_u32("x-ratelimit-remaining"),
            reset_unix: parse_i64("x-ratelimit-reset"),
        };
    }
}

async fn status_error(response: Response) -> AnalyzerError {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let message = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|value| {
            value
                .pointer("/errors/0/detail")
                .or_else(|| value.pointer("/errors/0/title"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| {
            if text.is_empty() {
                status
                    .canonical_reason()
                    .unwrap_or("unknown API error")
                    .to_owned()
            } else {
                text.chars().take(500).collect()
            }
        });
    AnalyzerError::ApiStatus {
        status: status.as_u16(),
        message,
    }
}

fn parse_match_document(document: &Value, platform: &str) -> Result<MatchMetadata> {
    let data = document
        .get("data")
        .and_then(Value::as_object)
        .ok_or_else(|| AnalyzerError::InvalidTelemetry("match response has no data".to_owned()))?;
    let match_id = data
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| AnalyzerError::InvalidTelemetry("match response has no ID".to_owned()))?
        .to_owned();
    let attributes = data.get("attributes").and_then(Value::as_object);
    let string_attr = |name: &str| {
        attributes
            .and_then(|value| value.get(name))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    };
    let duration_seconds = attributes
        .and_then(|value| value.get("duration"))
        .and_then(Value::as_i64);

    let asset_ids: Vec<&str> = data
        .get("relationships")
        .and_then(|value| value.get("assets"))
        .and_then(|value| value.get("data"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .collect();
    let telemetry_url = document
        .get("included")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|item| {
            item.get("type").and_then(Value::as_str) == Some("asset")
                && item
                    .get("id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| asset_ids.contains(&id))
        })
        .and_then(|item| item.pointer("/attributes/URL"))
        .and_then(Value::as_str)
        .ok_or_else(|| AnalyzerError::MissingTelemetry(match_id.clone()))?
        .to_owned();

    Ok(MatchMetadata {
        match_id,
        platform: platform.to_owned(),
        created_at: string_attr("createdAt"),
        duration_seconds,
        game_mode: string_attr("gameMode"),
        map_name: string_attr("mapName"),
        match_type: string_attr("matchType"),
        patch_version: string_attr("patchVersion"),
        telemetry_url,
    })
}

fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err(AnalyzerError::InvalidConfiguration(format!(
            "invalid {label}"
        )));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct PlayerDocument {
    data: Vec<PlayerResource>,
}

#[derive(Debug, Deserialize)]
struct PlayerResource {
    id: String,
    attributes: PlayerAttributes,
    relationships: PlayerRelationships,
}

#[derive(Debug, Deserialize)]
struct PlayerAttributes {
    name: String,
}

#[derive(Debug, Deserialize)]
struct PlayerRelationships {
    matches: MatchRelationship,
}

#[derive(Debug, Deserialize)]
struct MatchRelationship {
    data: Vec<ResourceIdentifier>,
}

#[derive(Debug, Deserialize)]
struct ResourceIdentifier {
    id: String,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_match_document;

    #[test]
    fn extracts_telemetry_asset_from_match_document() {
        let document = json!({
            "data": {
                "id": "match-1",
                "attributes": {
                    "createdAt": "2026-01-01T00:00:00Z",
                    "duration": 100,
                    "gameMode": "squad-fpp",
                    "mapName": "Baltic_Main",
                    "patchVersion": "40.1"
                },
                "relationships": {"assets": {"data": [{"type": "asset", "id": "asset-1"}]}}
            },
            "included": [{
                "type": "asset",
                "id": "asset-1",
                "attributes": {"URL": "https://telemetry-cdn.pubg.com/test.json"}
            }]
        });

        let parsed = parse_match_document(&document, "steam").expect("match should parse");
        assert_eq!(parsed.match_id, "match-1");
        assert_eq!(parsed.map_name.as_deref(), Some("Baltic_Main"));
        assert_eq!(
            parsed.telemetry_url,
            "https://telemetry-cdn.pubg.com/test.json"
        );
    }
}
