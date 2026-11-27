use std::time::Duration;

use serde_json::Value;

use super::model::{PublicIpDto, ToolErrorDto, ToolResult};

const PRIMARY_URL: &str = "https://ipwho.is/";
const FALLBACK_URL: &str = "https://ipinfo.io/json";

pub async fn lookup() -> ToolResult<PublicIpDto> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| ToolErrorDto::new("clientInit", error.to_string()))?;

    let primary = fetch(&client, PRIMARY_URL).await;
    match primary {
        Ok(value) => {
            if let Some(result) = parse_ipwho(&value) {
                return Ok(result);
            }
        }
        Err(error) => tracing::warn!(%error, "primary public IP provider failed"),
    }

    let fallback = fetch(&client, FALLBACK_URL).await.map_err(|error| {
        ToolErrorDto::new(
            "publicIpUnavailable",
            format!("No public IP provider responded: {error}"),
        )
    })?;
    parse_ipinfo(&fallback).ok_or_else(|| {
        ToolErrorDto::new(
            "publicIpUnavailable",
            "Public IP provider returned an unexpected payload",
        )
    })
}

async fn fetch(client: &reqwest::Client, url: &str) -> Result<Value, String> {
    let response = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    response
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn parse_ipwho(value: &Value) -> Option<PublicIpDto> {
    if value.get("success").and_then(Value::as_bool) == Some(false) {
        return None;
    }
    let ip = text(value, "ip")?;
    let connection = value.get("connection");
    let asn = connection
        .and_then(|item| item.get("asn"))
        .and_then(Value::as_u64)
        .map(|asn| format!("AS{asn}"));
    let timezone = value
        .get("timezone")
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| text(value, "timezone"));
    Some(PublicIpDto {
        ip,
        city: text(value, "city"),
        region: text(value, "region"),
        country: text(value, "country"),
        country_code: text(value, "country_code"),
        org: connection
            .and_then(|item| text(item, "org"))
            .or_else(|| connection.and_then(|item| text(item, "isp"))),
        asn,
        isp: connection.and_then(|item| text(item, "isp")),
        timezone,
        latitude: number(value, "latitude"),
        longitude: number(value, "longitude"),
        provider: "ipwho.is".to_owned(),
    })
}

fn parse_ipinfo(value: &Value) -> Option<PublicIpDto> {
    let ip = text(value, "ip")?;
    let org = text(value, "org");
    let asn = org
        .as_deref()
        .and_then(|value| value.split_whitespace().next())
        .filter(|token| token.to_ascii_uppercase().starts_with("AS"))
        .map(str::to_owned);
    let (latitude, longitude) = text(value, "loc")
        .and_then(|loc| {
            let (lat, lon) = loc.split_once(',')?;
            Some((lat.parse::<f64>().ok()?, lon.parse::<f64>().ok()?))
        })
        .map_or((None, None), |(lat, lon)| (Some(lat), Some(lon)));
    Some(PublicIpDto {
        ip,
        city: text(value, "city"),
        region: text(value, "region"),
        country: text(value, "country"),
        country_code: text(value, "country"),
        org: org.clone(),
        asn,
        isp: None,
        timezone: text(value, "timezone"),
        latitude,
        longitude,
        provider: "ipinfo.io".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_ipwho_payload() {
        let payload = json!({
            "success": true,
            "ip": "203.0.113.7",
            "city": "Shanghai",
            "region": "Shanghai",
            "country": "China",
            "country_code": "CN",
            "latitude": 31.22,
            "longitude": 121.46,
            "connection": { "asn": 4812, "org": "China Telecom", "isp": "China Telecom" },
            "timezone": { "id": "Asia/Shanghai" }
        });
        let parsed = parse_ipwho(&payload).expect("payload");
        assert_eq!(parsed.ip, "203.0.113.7");
        assert_eq!(parsed.asn.as_deref(), Some("AS4812"));
        assert_eq!(parsed.timezone.as_deref(), Some("Asia/Shanghai"));
        assert_eq!(parsed.provider, "ipwho.is");
    }

    #[test]
    fn parses_ipinfo_payload() {
        let payload = json!({
            "ip": "198.51.100.4",
            "city": "Tokyo",
            "region": "Tokyo",
            "country": "JP",
            "loc": "35.68,139.76",
            "org": "AS2516 KDDI CORPORATION",
            "timezone": "Asia/Tokyo"
        });
        let parsed = parse_ipinfo(&payload).expect("payload");
        assert_eq!(parsed.asn.as_deref(), Some("AS2516"));
        assert_eq!(parsed.latitude, Some(35.68));
        assert_eq!(parsed.provider, "ipinfo.io");
    }

    #[test]
    fn rejects_unsuccessful_ipwho_payload() {
        let payload = json!({ "success": false, "message": "reserved range" });
        assert!(parse_ipwho(&payload).is_none());
    }
}
