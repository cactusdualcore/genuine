use std::fmt;

use http::Response;
use hyper::body::Bytes;

use crate::request::Request;

pub trait IntoResponse {
    fn into_response(self) -> Response<Bytes>;
}

impl<B: Into<Bytes>> IntoResponse for Response<B> {
    fn into_response(self) -> Response<Bytes> {
        self.map(Into::into)
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response<Bytes> {
        let bytes = Bytes::from(self);
        Response::new(bytes)
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Response<Bytes> {
        let bytes = Bytes::from_static(self.as_bytes());
        Response::new(bytes)
    }
}

pub trait Handler: Send + Sync + 'static {
    fn handle_request(&self, req: Request) -> Response<Bytes>;
}

pub trait FromRequest: Sized {
    type Error: fmt::Debug;

    fn from_request(req: &Request) -> Result<Self, Self::Error>;
}

impl<R> Handler for fn() -> R
where
    Self: Send + 'static,
    R: IntoResponse,
{
    fn handle_request(&self, _: Request) -> Response<Bytes> {
        self().into_response()
    }
}

impl<P1, P2, R> Handler for fn(P1, P2) -> R
where
    Self: Send + 'static,
    R: IntoResponse,
    P1: FromRequest,
    P2: FromRequest,
{
    fn handle_request<'a>(&'a self, req: Request) -> Response<Bytes> {
        let p1 = P1::from_request(&req).unwrap();
        let p2 = P2::from_request(&req).unwrap();
        self(p1, p2).into_response()
    }
}
