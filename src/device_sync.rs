use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::header::{CONTENT_TYPE, IF_NONE_MATCH};
use serde::{Deserialize, Serialize};

const DEFAULT_RUNTIME_VERSION: &str = "0.1.0";

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LatestPackage {
    pub schema_version: i64,
    pub revision: i64,
    pub package_id: String,
    pub created_at: String,
    pub archive_size: i64,
    pub archive_sha256: String,
    pub minimum_runtime_version: String,
    pub download_url: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PackageAcknowledgement {
    pub status: String,
    pub package_id: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PackageFailure {
    pub status: String,
    pub package_id: String,
}

#[derive(Debug)]
pub struct DeviceSyncClient {
    client: reqwest::blocking::Client,
    base_url: reqwest::Url,
    device_id: String,
    token: String,
}

impl DeviceSyncClient {
    pub fn new(
        base_url: &str,
        device_id: impl Into<String>,
        token: impl Into<String>,
        ca_cert_path: Option<&Path>,
    ) -> Result<Self> {
        let base_url = validate_sync_base_url(base_url)?;
        let client = sync_http_client(ca_cert_path)?;
        Ok(Self {
            client,
            base_url,
            device_id: device_id.into(),
            token: token.into(),
        })
    }

    pub fn latest(&self, etag: Option<&str>) -> Result<Option<LatestPackage>> {
        let mut request =
            self.authorized_request(self.client.get(self.endpoint("/content/latest")));
        if let Some(etag) = etag {
            request = request.header(IF_NONE_MATCH, etag);
        }
        let response = request.send().context("failed to fetch latest package")?;
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(None);
        }
        let response = response.error_for_status()?;
        let body = response
            .bytes()
            .context("failed to read latest package response")?;
        Ok(Some(
            serde_json::from_slice::<LatestPackage>(&body)
                .context("failed to parse latest package response")?,
        ))
    }

    pub fn download(&self, package_id: &str) -> Result<Vec<u8>> {
        let response = self
            .authorized_request(
                self.client
                    .get(self.endpoint(&format!("/content/packages/{package_id}"))),
            )
            .send()
            .context("failed to download package")?
            .error_for_status()?;
        response
            .bytes()
            .map(|bytes| bytes.to_vec())
            .context("failed to read package download")
    }

    pub fn acknowledge_activation(
        &self,
        package_id: &str,
        runtime_version: &str,
    ) -> Result<PackageAcknowledgement> {
        self.post_ack(package_id, runtime_version)
    }

    pub fn report_failure(
        &self,
        package_id: &str,
        runtime_version: &str,
        stage: &str,
        detail: &str,
    ) -> Result<PackageFailure> {
        let response = self
            .authorized_request(
                self.client
                    .post(self.endpoint(&format!("/content/packages/{package_id}/failed"))),
            )
            .header(CONTENT_TYPE, "application/json")
            .body(
                serde_json::to_vec(&serde_json::json!({
                    "runtime_version": runtime_version,
                    "stage": stage,
                    "detail": detail,
                }))
                .context("failed to encode package failure request")?,
            )
            .send()
            .context("failed to report package failure")?
            .error_for_status()?;
        let body = response
            .bytes()
            .context("failed to read package failure acknowledgement")?;
        serde_json::from_slice::<PackageFailure>(&body)
            .context("failed to parse package failure acknowledgement")
    }

    pub fn runtime_version() -> &'static str {
        DEFAULT_RUNTIME_VERSION
    }

    fn post_ack(&self, package_id: &str, runtime_version: &str) -> Result<PackageAcknowledgement> {
        let response = self
            .authorized_request(
                self.client
                    .post(self.endpoint(&format!("/content/packages/{package_id}/activated"))),
            )
            .header(CONTENT_TYPE, "application/json")
            .body(
                serde_json::to_vec(&serde_json::json!({ "runtime_version": runtime_version }))
                    .context("failed to encode package activation request")?,
            )
            .send()
            .context("failed to acknowledge package activation")?
            .error_for_status()?;
        let body = response
            .bytes()
            .context("failed to read package activation acknowledgement")?;
        serde_json::from_slice::<PackageAcknowledgement>(&body)
            .context("failed to parse package activation acknowledgement")
    }

    fn endpoint(&self, path: &str) -> reqwest::Url {
        self.base_url.join(path).expect("valid sync base url")
    }

    fn authorized_request(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        request
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.token),
            )
            .header("X-TCube-Device-ID", &self.device_id)
    }
}

fn sync_http_client(ca_cert_path: Option<&Path>) -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(30));
    if let Some(path) = ca_cert_path {
        let pem = fs::read(path).with_context(|| {
            format!(
                "failed to read device API CA certificate {}",
                path.display()
            )
        })?;
        let certificate = reqwest::Certificate::from_pem(&pem).with_context(|| {
            format!(
                "failed to parse device API CA certificate {}",
                path.display()
            )
        })?;
        builder = builder.add_root_certificate(certificate);
    }
    builder
        .build()
        .context("failed to build device sync HTTP client")
}

fn validate_sync_base_url(url: &str) -> Result<reqwest::Url> {
    let parsed =
        reqwest::Url::parse(url).with_context(|| format!("invalid device sync base URL: {url}"))?;
    if parsed.scheme() != "https" {
        anyhow::bail!("device sync base URL must use https: {url}");
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;
    use tempfile::TempDir;

    #[test]
    fn sync_base_url_must_use_https() {
        assert!(validate_sync_base_url("https://localhost/api").is_ok());
        assert!(validate_sync_base_url("http://localhost/api").is_err());
    }

    #[test]
    fn sync_client_loads_custom_root_certificate() {
        let dir = TempDir::new().unwrap();
        let cert_path = dir.path().join("device-api-ca.crt");
        write(
            &cert_path,
            b"-----BEGIN CERTIFICATE-----\nMIICpDCCAYwCCQDpAesS5Rc0YzANBgkqhkiG9w0BAQsFADAUMRIwEAYDVQQDDAls\nb2NhbGhvc3QwHhcNMjYwNjIyMTEyMzMyWhcNMjYwNjIzMTEyMzMyWjAUMRIwEAYD\nVQQDDAlsb2NhbGhvc3QwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDZ\nYlRHQ24BleueDVCphzdU7ONSyLlcrR4cDlQp9ayS6z4R3ORxz18FVdABXBzBlOT6\njNLRacsgTZLOra4r+eQclls8PWj6OWkq6jFfjzYJI13rjJEwdX+k49i2riUgS3n3\nwSr7LIn56moi2r8AmGD7mZKijNXODAQ+rIT8DKKpiw7igbghUsHhD5LOZMiqNGoB\n1XGFZmYPq0F1E1rNVzpl2PEVBWxUNk9DiQPvUGNGwlcfBEniH5dfCuDfAUYeHBLY\nIPT69KoSeCoBShSvMGgewIQz16+783QAOzmC5brAZgrlKeCCNFx7QjrTouWZ1MK0\nMs+YcoQFHoEgenCs9RnZAgMBAAEwDQYJKoZIhvcNAQELBQADggEBABX4bq6VntHb\n0y52sA8w11qMR81S5IemcDzQhdwBN7Oe8Sdg3pu1xM+BuMxfmbYVP20Lt1SKIm96\n5Yuq8vjhYtvYHDFU5qkTg5vmyrJ0C+HZSlDSzGYHTKuS1tjmTOpZkUZU+SM3bXXi\nmgqwVxJ9W0dCKyKJaI5A0uPbwuGkwmOxPMoy+pqPeDY+tHrJ/bp66ew/4K2g4SDz\n/tyIpaKcKngpaVxmrml7pZ11CobuuPznIL9EGkzJQ3VRFs6CmKAbkV5X1Fx6Q1Ok\ntpfBGYghLsPt5k32bp/4+oaxGOBEV5DNSSKb8MA+dvmwWJXq0QW8G56fHlsI1q9b\nWrcxxfJMPHk=\n-----END CERTIFICATE-----\n",
        )
        .unwrap();
        let client = sync_http_client(Some(&cert_path)).unwrap();
        let request = client
            .get(validate_sync_base_url("https://example.com/api").unwrap())
            .build();
        assert!(request.is_ok());
    }
}
