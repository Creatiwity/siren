use crate::models::update_metadata::common::SyntheticGroupType;
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

#[derive(Clone)]
pub struct SentryCrons {
    url: String,
    client: Client,
}

#[derive(Serialize)]
struct CheckInBody {
    status: &'static str,
    monitor_config: MonitorConfig,
}

#[derive(Serialize)]
struct MonitorConfig {
    schedule: Schedule,
    checkin_margin: u32,
    max_runtime: u32,
    timezone: &'static str,
    failure_issue_threshold: u32,
    recovery_threshold: u32,
}

#[derive(Serialize)]
struct Schedule {
    #[serde(rename = "type")]
    schedule_type: &'static str,
    value: String,
}

fn group_type_slug(group_type: SyntheticGroupType) -> &'static str {
    match group_type {
        SyntheticGroupType::UnitesLegales => "unites_legales",
        SyntheticGroupType::Etablissements => "etablissements",
        SyntheticGroupType::LiensSuccession => "liens_succession",
        SyntheticGroupType::SirenDoublons => "siren_doublons",
        SyntheticGroupType::All => "all",
    }
}

// dsn: https://PUBLIC_KEY@HOST/PROJECT_ID
fn parse_dsn(dsn: &str) -> Option<(String, String, String)> {
    let (scheme, rest) = if let Some(r) = dsn.strip_prefix("https://") {
        ("https", r)
    } else if let Some(r) = dsn.strip_prefix("http://") {
        ("http", r)
    } else {
        return None;
    };
    let at_pos = rest.find('@')?;
    let public_key = rest[..at_pos].to_string();
    let after_at = &rest[at_pos + 1..];
    let slash_pos = after_at.find('/')?;
    let host = &after_at[..slash_pos];
    let project_id = after_at[slash_pos + 1..].to_string();
    let ingest = format!("{}://{}", scheme, host);
    Some((ingest, project_id, public_key))
}

impl SentryCrons {
    pub fn for_update(group_type: SyntheticGroupType) -> Option<Self> {
        let dsn = std::env::var("SENTRY_DSN").ok()?;
        let (ingest, project_id, public_key) = parse_dsn(&dsn)?;
        let slug = format!("update-{}", group_type_slug(group_type));
        let url = format!(
            "{}/api/{}/cron/{}/{}/",
            ingest, project_id, slug, public_key
        );

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()
            .ok()?;

        Some(Self { url, client })
    }

    pub async fn notify_in_progress(self, crontab: String) {
        let body = CheckInBody {
            status: "in_progress",
            monitor_config: MonitorConfig {
                schedule: Schedule {
                    schedule_type: "crontab",
                    value: crontab,
                },
                checkin_margin: 5,
                max_runtime: 120,
                timezone: "UTC",
                failure_issue_threshold: 1,
                recovery_threshold: 1,
            },
        };

        if let Err(e) = self.client.post(&self.url).json(&body).send().await {
            tracing::warn!("Sentry Crons in_progress check-in failed: {e}");
        }
    }

    pub async fn notify_ok(self) {
        if let Err(e) = self
            .client
            .get(format!("{}?status=ok", self.url))
            .send()
            .await
        {
            tracing::warn!("Sentry Crons ok check-in failed: {e}");
        }
    }

    pub async fn notify_error(self) {
        if let Err(e) = self
            .client
            .get(format!("{}?status=error", self.url))
            .send()
            .await
        {
            tracing::warn!("Sentry Crons error check-in failed: {e}");
        }
    }
}
