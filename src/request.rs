use hyper::body::Bytes;

use crate::router::routes::Match;

type HttpRequest = http::Request<Bytes>;

#[derive(Debug)]
pub struct Request {
    request: HttpRequest,
    matches: Vec<Match>,
}

impl Request {
    pub fn new(request: HttpRequest, matches: Vec<Match>) -> Self {
        Self { request, matches }
    }

    pub fn matches(&self) -> &[Match] {
        self.matches.as_slice()
    }
}

impl std::ops::Deref for Request {
    type Target = HttpRequest;

    fn deref(&self) -> &Self::Target {
        &self.request
    }
}
