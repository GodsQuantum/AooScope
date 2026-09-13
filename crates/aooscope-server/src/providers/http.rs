use futures_util::StreamExt;
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use std::time::Duration;

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
}

#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("request failed")]
    Request,
    #[error("provider returned an unsuccessful response")]
    Status,
    #[error("provider response was invalid")]
    Response,
}

impl HttpClient {
    pub fn new(verify_tls: bool) -> Result<Self, HttpError> {
        Client::builder()
            .timeout(Duration::from_secs(4))
            .danger_accept_invalid_certs(!verify_tls)
            .user_agent("aooscope/0.3")
            .build()
            .map(|client| Self { client })
            .map_err(|_| HttpError::Request)
    }

    pub async fn get_json(
        &self,
        base: &str,
        path: &str,
        headers: &[(&str, &str)],
    ) -> Result<Value, HttpError> {
        let response = self
            .request(self.client.get(url(base, path)), headers)
            .send()
            .await?;
        json_response(response).await
    }

    pub async fn post_form(
        &self,
        base: &str,
        path: &str,
        form: &[(&str, &str)],
        headers: &[(&str, &str)],
    ) -> Result<Response, HttpError> {
        self.request(self.client.post(url(base, path)).form(form), headers)
            .send()
            .await
            .map_err(|_| HttpError::Request)
    }

    fn request(
        &self,
        mut request: reqwest::RequestBuilder,
        headers: &[(&str, &str)],
    ) -> reqwest::RequestBuilder {
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        request
    }
}

impl From<reqwest::Error> for HttpError {
    fn from(error: reqwest::Error) -> Self {
        if error
            .status()
            .is_some_and(|status| status != StatusCode::OK)
        {
            Self::Status
        } else {
            Self::Request
        }
    }
}

async fn json_response(response: Response) -> Result<Value, HttpError> {
    if !response.status().is_success() {
        return Err(HttpError::Status);
    }
    let bytes = bounded_body(response, MAX_RESPONSE_BYTES).await?;
    serde_json::from_slice(&bytes).map_err(|_| HttpError::Response)
}

pub async fn bounded_body(response: Response, limit: usize) -> Result<Vec<u8>, HttpError> {
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| HttpError::Request)?;
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(HttpError::Response);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub fn url(base: &str, path: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    let base = if base.contains("://") {
        base.to_owned()
    } else {
        format!("http://{base}")
    };
    format!("{base}/{}", path.trim_start_matches('/'))
}

pub fn cookie_from_response(response: &Response) -> Option<String> {
    let cookie = response
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("; ");
    (!cookie.is_empty()).then_some(cookie)
}
