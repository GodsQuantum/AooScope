use axum::{
    body::Body,
    extract::Path,
    http::{HeaderMap, HeaderValue, Response, StatusCode, header},
    response::IntoResponse,
};
use rust_embed_for_web::{EmbedableFile, RustEmbed};

#[derive(RustEmbed)]
#[folder = "../../frontend/build/"]
#[exclude = "**/*.br"]
#[exclude = "**/*.gz"]
struct FrontendAssets;

pub async fn index(headers: HeaderMap) -> Response<Body> {
    serve("index.html", &headers)
}

pub async fn asset(Path(path): Path<String>, headers: HeaderMap) -> Response<Body> {
    serve(path.trim_start_matches('/'), &headers)
}

fn serve(path: &str, request_headers: &HeaderMap) -> Response<Body> {
    let Some(file) = FrontendAssets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let etag = meta_string(file.etag());
    if request_headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        == Some(etag.as_str())
    {
        return not_modified(&file, path, etag);
    }
    let accepted = request_headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let (data, encoding) = if accepted.split(',').any(|v| v.trim().starts_with("br")) {
        file.data_br()
            .map(|d| (d, Some("br")))
            .unwrap_or_else(|| (file.data(), None))
    } else if accepted.split(',').any(|v| v.trim().starts_with("gzip")) {
        file.data_gzip()
            .map(|d| (d, Some("gzip")))
            .unwrap_or_else(|| (file.data(), None))
    } else {
        (file.data(), None)
    };

    let mut response = Response::new(Body::from(data_vec(data)));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(header::ETAG, HeaderValue::from_str(&etag).unwrap());
    headers.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control(path)),
    );
    if let Some(value) = file.last_modified()
        && let Ok(value) = HeaderValue::from_str(value.as_ref())
    {
        headers.insert(header::LAST_MODIFIED, value);
    }
    if let Some(mime) = file.mime_type()
        && let Ok(value) = HeaderValue::from_str(mime.as_ref())
    {
        headers.insert(header::CONTENT_TYPE, value);
    }
    if let Some(encoding) = encoding {
        headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static(encoding));
    }
    response
}
fn cache_control(path: &str) -> &'static str {
    if path.starts_with("_app/immutable/") {
        "public,max-age=31536000,immutable"
    } else {
        "no-cache"
    }
}

fn not_modified<F: EmbedableFile>(file: &F, path: &str, etag: String) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_MODIFIED;
    let headers = response.headers_mut();
    headers.insert(header::ETAG, HeaderValue::from_str(&etag).unwrap());
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control(path)),
    );
    headers.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    if let Some(value) = file.last_modified()
        && let Ok(value) = HeaderValue::from_str(value.as_ref())
    {
        headers.insert(header::LAST_MODIFIED, value);
    }
    response
}

fn meta_string<T: AsRef<str>>(value: T) -> String {
    value.as_ref().to_owned()
}

fn data_vec<T: AsRef<[u8]>>(value: T) -> Vec<u8> {
    value.as_ref().to_vec()
}
