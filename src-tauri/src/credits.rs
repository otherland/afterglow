use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Credits {
    pub plan: String,
    pub resets: String,
    pub credits: Snapshot,
}

#[derive(Debug, Deserialize)]
pub struct Snapshot {
    pub quota_remaining: f64,
    pub entitlement: f64,
    pub credits_used: f64,
    pub percent_remaining: f64,
    pub timestamp_utc: Option<String>,
}

pub fn parse(input: &[u8]) -> Result<Credits, String> {
    let value: Value =
        serde_json::from_slice(input).map_err(|e| format!("Invalid API JSON: {e}"))?;
    if value.pointer("/credits/unlimited").and_then(Value::as_bool) == Some(true) {
        return Err(
            "This account has unlimited quota; no finite monthly balance is available.".into(),
        );
    }
    let result: Credits = serde_json::from_value(value)
        .map_err(|e| format!("Missing or invalid quota field: {e}"))?;
    for (name, value) in [
        ("quota_remaining", result.credits.quota_remaining),
        ("entitlement", result.credits.entitlement),
        ("credits_used", result.credits.credits_used),
        ("percent_remaining", result.credits.percent_remaining),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!(
                "Invalid {name}: expected a non-negative finite number"
            ));
        }
    }
    if result.credits.percent_remaining > 100.0 {
        return Err("Invalid percent_remaining: greater than 100".into());
    }
    if result.plan.trim().is_empty() || result.resets.trim().is_empty() {
        return Err("Missing plan or quota_reset_date".into());
    }
    Ok(result)
}

pub fn level(percent: f64) -> &'static str {
    if percent < 10.0 {
        "Critical"
    } else if percent < 20.0 {
        "Low"
    } else {
        "Remaining"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn response() -> Value {
        json!({
            "plan": "enterprise", "resets": "2026-11-01",
            "credits": {
                "quota_remaining": 1250.5, "entitlement": 4000,
                "credits_used": 2749, "percent_remaining": 31.3,
                "unlimited": false
            }
        })
    }

    #[test]
    fn preserves_fractional_credits_without_recalculating() {
        let data = parse(&serde_json::to_vec(&response()).unwrap()).unwrap();
        assert_eq!(data.credits.quota_remaining, 1250.5);
        assert_eq!(data.credits.credits_used, 2749.0);
        assert_eq!(data.credits.percent_remaining, 31.3);
    }

    #[test]
    fn missing_fields_never_become_zero() {
        for name in [
            "quota_remaining",
            "entitlement",
            "credits_used",
            "percent_remaining",
        ] {
            let mut value = response();
            value["credits"].as_object_mut().unwrap().remove(name);
            assert!(parse(&serde_json::to_vec(&value).unwrap())
                .unwrap_err()
                .contains(name));
        }
    }

    #[test]
    fn rejects_invalid_responses() {
        assert!(parse(b"not json").is_err());
        for bad in [Value::Null, json!("55"), json!(-1), json!(101)] {
            let mut value = response();
            value["credits"]["percent_remaining"] = bad;
            assert!(parse(&serde_json::to_vec(&value).unwrap()).is_err());
        }
        let mut value = response();
        value["credits"]["unlimited"] = json!(true);
        assert!(parse(&serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("unlimited"));
    }

    #[test]
    fn thresholds_are_strictly_below_twenty_and_ten() {
        assert_eq!(level(0.0), "Critical");
        assert_eq!(level(9.9), "Critical");
        assert_eq!(level(10.0), "Low");
        assert_eq!(level(19.9), "Low");
        assert_eq!(level(20.0), "Remaining");
    }
}
