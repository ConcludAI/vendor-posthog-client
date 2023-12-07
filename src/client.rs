use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use reqwest::header::CONTENT_TYPE;

const API_ENDPOINT: &str = "https://app.posthog.com/";
const APT_CAPTURE: &str = "capture/";
const TIMEOUT: Duration = Duration::from_millis(8000);
const POSTHOG_ENV: &str = "POSTHOG_API_KEY";

pub struct ApiOptions {
    endpoint: String,
    key: String,
}

pub struct Client {
    options: ApiOptions,
    client: reqwest::blocking::Client,
}

#[derive(serde::Serialize, Debug, PartialEq, Eq)]
pub struct Event {
    event: String,
    properties: Properties,
    timestamp: Option<chrono::NaiveDateTime>,
}

#[derive(serde::Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Properties {
    distinct_id: String,
    properties: HashMap<String, String>,
}

#[derive(Serialize)]
struct InnerEvent {
    api_key: String,
    event: String,
    properties: Properties,
    timestamp: Option<chrono::NaiveDateTime>,
}

impl ApiOptions {
    pub fn new(endpoint: String, key: String) -> ApiOptions {
        ApiOptions { endpoint, key }
    }

    pub fn from_env() -> Result<ApiOptions> {
        let key = std::env::var(POSTHOG_ENV)?;
        Ok(ApiOptions::new(API_ENDPOINT.to_string(), key))
    }

    pub fn from_google_secret_manager() -> Result<ApiOptions> {
        todo!("Implement me!")
    }

    pub fn auto() -> Result<ApiOptions> {
        match ApiOptions::from_env() {
            Ok(options) => Ok(options),
            Err(_) => match ApiOptions::from_google_secret_manager() {
                Ok(options) => Ok(options),
                Err(e) => Err(e),
            },
        }
    }
}

impl Client {
    pub fn new(options: ApiOptions) -> Client {
        let client = reqwest::blocking::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .unwrap();
        Client { options, client}
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap();
    }

    pub fn capture(&self, event: Event) -> Result<()> {
        let inner_event = InnerEvent::new(event, self.options.key.clone());
        let url = format!(
            "{}{}",
            self.options.endpoint, APT_CAPTURE
        );
        let _response = self
            .client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(serde_json::to_string(&inner_event)?)
            .send()
            .map_err(|e| anyhow::anyhow!(e))?;

        println!("{:?}", _response);

        Ok(())
    }

    pub fn capture_batch(&self, events: Vec<Event>) -> Result<()> {
        for event in events {
            self.capture(event)?;
        }

        Ok(())
    }

}

impl Event {
    pub fn new(event: String, distinct_id: String) -> Event {
        Event {
            event,
            properties: Properties::new(distinct_id),
            timestamp: None,
        }
    }

    pub fn insert_prop(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }

    pub fn insert_prop_many(&mut self, props: Vec<(String, String)>) {
        props.into_iter().for_each(|(key, value)| {
            self.properties.insert(key, value);
        });
    }

    pub fn set_timestamp(&mut self, timestamp: chrono::NaiveDateTime) {
        self.timestamp = Some(timestamp);
    }
}

impl InnerEvent {
    pub fn new(event: Event, api_key: String) -> InnerEvent {
        InnerEvent {
            api_key,
            event: event.event,
            properties: event.properties,
            timestamp: event.timestamp,
        }
    }
}

impl Properties {
    pub fn new(distinct_id: String) -> Properties {
        Properties {
            distinct_id,
            properties: HashMap::default(),
        }
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    fn test_client(client: &Client) {
        let mut event = Event::new("user_logged_in".to_string(), "distinct_id_user".to_string());
        event.insert_prop("key".to_string(), "value".to_string());
        event.insert_prop_many(vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]);
        event.set_timestamp(chrono::Utc::now().naive_utc());
        client.capture(event).unwrap();
    }

    #[test]
    fn inner_event_serializes() {
        let mut event = Event::new("event".to_string(), "distinct_id".to_string());
        event.insert_prop("key".to_string(), "value".to_string());
        let inner_event = InnerEvent::new(event, "api_key".to_string());
        let json = serde_json::to_value(&inner_event).unwrap();
        let assert_json = "{\"api_key\":\"api_key\",\"event\":\"event\",\"properties\":{\"distinct_id\":\"distinct_id\",\"properties\":{\"key\":\"value\"}},\"timestamp\":null}";
        assert_eq!(json, assert_json.parse::<serde_json::Value>().unwrap());
    }

    #[test]
    fn test_client_env() {
        let opts = ApiOptions::from_env();
        assert!(opts.is_ok());
        let opts = opts.unwrap();
        let client = Client::new(opts);
        test_client(&client);
    }

    #[test]
    fn test_client_google_secret_manager() {
        let opts = ApiOptions::from_google_secret_manager();
        assert!(opts.is_err());
        let opts = opts.unwrap();
        let client = Client::new(opts);
        test_client(&client);
    }
}
