//! Typed client for the `<app>` HTTP API. ponytail: mirror the wire contract
//! defined by `<app>-api` here so callers get compile-time-checked requests.
use <app>_core::Result;

#[derive(Debug, Clone)]
pub struct Client {
    base: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(base: String) -> Self {
        Self {
            base,
            http: reqwest::Client::new(),
        }
    }

    /// ponytail: implement typed endpoints. Map reqwest errors into
    /// `<app>_core::Error::Upstream`.
    pub async fn health(&self) -> Result<bool> {
        let url = format!("{}/healthz", self.base);
        let res =
            self.http
                .get(&url)
                .send()
                .await
                .map_err(|e| <app>_core::Error::Upstream {
                    service: "api",
                    detail: e.to_string(),
                })?;
        Ok(res.status().is_success())
    }
}
