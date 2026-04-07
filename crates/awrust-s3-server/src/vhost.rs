use axum::extract::Request;
use axum::http::Uri;
use axum::response::Response;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::Service;

fn extract_bucket<'a>(host: &'a str, base_domain: &str) -> Option<&'a str> {
    let host = host.split(':').next()?;
    let prefix = host.strip_suffix(base_domain)?.strip_suffix('.')?;
    if prefix.is_empty() {
        return None;
    }
    Some(prefix)
}

#[derive(Clone)]
pub struct VhostService<S> {
    inner: S,
    base_domain: Arc<String>,
}

impl<S> VhostService<S> {
    pub fn new(inner: S, base_domain: Arc<String>) -> Self {
        Self { inner, base_domain }
    }
}

impl<S> Service<Request> for VhostService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let bucket = req
            .headers()
            .get("host")
            .and_then(|v| v.to_str().ok())
            .and_then(|host| extract_bucket(host, &self.base_domain))
            .map(str::to_owned);

        let req = if let Some(bucket) = bucket {
            let path = req.uri().path();
            let query = req.uri().query().map(str::to_owned);
            let suffix = path.strip_prefix('/').unwrap_or(path);
            let new_path = if suffix.is_empty() {
                format!("/{bucket}")
            } else {
                format!("/{bucket}/{suffix}")
            };
            let new_uri: Uri = match query {
                Some(q) => format!("{new_path}?{q}"),
                None => new_path,
            }
            .parse()
            .expect("valid rewritten URI");

            let (mut parts, body) = req.into_parts();
            parts.uri = new_uri;
            Request::from_parts(parts, body)
        } else {
            req
        };

        let fut = self.inner.call(req);
        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subdomain_extracted() {
        assert_eq!(
            extract_bucket("mybucket.localhost", "localhost"),
            Some("mybucket")
        );
    }

    #[test]
    fn subdomain_with_port() {
        assert_eq!(
            extract_bucket("mybucket.localhost:4566", "localhost"),
            Some("mybucket")
        );
    }

    #[test]
    fn bare_host_returns_none() {
        assert_eq!(extract_bucket("localhost", "localhost"), None);
    }

    #[test]
    fn bare_host_with_port_returns_none() {
        assert_eq!(extract_bucket("localhost:4566", "localhost"), None);
    }

    #[test]
    fn deep_subdomain() {
        assert_eq!(
            extract_bucket("deep.sub.localhost", "localhost"),
            Some("deep.sub")
        );
    }

    #[test]
    fn custom_base_domain() {
        assert_eq!(
            extract_bucket("mybucket.example.com", "example.com"),
            Some("mybucket")
        );
    }

    #[test]
    fn custom_base_domain_with_port() {
        assert_eq!(
            extract_bucket("mybucket.example.com:9000", "example.com"),
            Some("mybucket")
        );
    }

    #[test]
    fn unrelated_host_returns_none() {
        assert_eq!(extract_bucket("other.com:4566", "localhost"), None);
    }

    #[test]
    fn empty_host_returns_none() {
        assert_eq!(extract_bucket("", "localhost"), None);
    }
}
