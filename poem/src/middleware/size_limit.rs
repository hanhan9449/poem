use crate::{
    error::SizedLimitError, web::headers::HeaderMapExt, Endpoint, Middleware, Request, Result,
};

/// Middleware for limit the request payload size.
///
/// If the incoming request does not contain the `Content-Length` header, it
/// will return `BAD_REQUEST` status code.
///
/// # Errors
///
/// - [`SizedLimitError`]
pub struct SizeLimit {
    max_size: usize,
}

impl SizeLimit {
    /// Create `SizeLimit` middleware.
    pub fn new(max_size: usize) -> Self {
        Self { max_size }
    }
}

impl<E: Endpoint> Middleware<E> for SizeLimit {
    type Output = SizeLimitEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        SizeLimitEndpoint {
            inner: ep,
            max_size: self.max_size,
        }
    }
}

/// Endpoint for SizeLimit middleware.
pub struct SizeLimitEndpoint<E> {
    inner: E,
    max_size: usize,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for SizeLimitEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let content_length = req
            .headers()
            .typed_get::<headers::ContentLength>()
            .ok_or(SizedLimitError::MissingContentLength)?;

        if content_length.0 as usize > self.max_size {
            return Err(SizedLimitError::PayloadTooLarge.into());
        }

        self.inner.call(req).await
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use super::*;
    use crate::{
        endpoint::{make_sync, EndpointExt},
        IntoResponse,
    };

    #[tokio::test]
    async fn size_limit() {
        let ep = make_sync(|_| ()).with(SizeLimit::new(5));

        assert_eq!(
            ep.call(Request::builder().body(&b"123456"[..]))
                .await
                .unwrap_err()
                .downcast_ref::<SizedLimitError>(),
            Some(&SizedLimitError::MissingContentLength)
        );

        assert_eq!(
            ep.call(
                Request::builder()
                    .header("content-length", 6)
                    .body(&b"123456"[..])
            )
            .await
            .unwrap_err()
            .downcast_ref::<SizedLimitError>(),
            Some(&SizedLimitError::PayloadTooLarge)
        );

        assert_eq!(
            ep.call(
                Request::builder()
                    .header("content-length", 4)
                    .body(&b"1234"[..])
            )
            .await
            .unwrap()
            .into_response()
            .status(),
            StatusCode::OK
        );

        assert_eq!(
            ep.call(
                Request::builder()
                    .header("content-length", 5)
                    .body(&b"12345"[..])
            )
            .await
            .unwrap()
            .into_response()
            .status(),
            StatusCode::OK
        );
    }
}
