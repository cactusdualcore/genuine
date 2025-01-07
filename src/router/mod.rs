pub mod groups;
pub mod middleware;
pub mod routes;

use groups::Group;
use http::{Method, Request, Response, Uri};
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{
    body::{Body, Bytes, Incoming},
    service::Service,
};
use middleware::BoxedMiddleware;
use routes::{Match, Route};

type BoxedFuture<T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + Sync + 'static>>;

pub struct Router {
    pub(crate) groups: Vec<Group>,
    begin: Vec<BoxedMiddleware>,
    before: Vec<BoxedMiddleware>,
    after: Vec<BoxedMiddleware>,
    finish: Vec<BoxedMiddleware>,
}

#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error("invalid request body")]
    InvalidBody,
}

impl Router {
    pub fn new() -> Self {
        Self {
            groups: vec![Group::new("")],
            begin: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            finish: Vec::new(),
        }
    }

    fn route(&self, uri: &Uri, method: &Method) -> Option<(&Group, &Route, Vec<Match>)> {
        let path = match uri.path() {
            path @ "/" => path,
            path => path.trim_end_matches("/"),
        }
        .to_owned();

        self.groups
            .iter()
            .filter_map(|g| g.routes.get(method).map(|routes| (g, routes)))
            .find_map(|(g, routes)| {
                routes
                    .iter()
                    .find_map(|route| route.path.matches(&path).map(|matches| (g, route, matches)))
            })
    }
}

impl Service<Request<Incoming>> for Router {
    type Response = Response<BoxBody<Bytes, hyper::Error>>;
    type Error = RouterError;
    type Future = BoxedFuture<Result<Self::Response, Self::Error>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        for _begin in &self.begin {}

        // SAFETY: Router must outlive future
        let router: &'static Router = unsafe { std::mem::transmute(self) };

        let fut: Self::Future = match router.route(req.uri(), req.method()) {
            Some((group, route, matches)) => Box::pin(async move {
                let Some(req) = ensure_max_body_size(req) else {
                    let mut res = Response::new(full("Body too big".into()));
                    *res.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
                    return Ok(res);
                };

                let req = collect_full_request_body(req).await?;

                let req = crate::request::Request::new(req, matches);

                for _before in &router.before {}
                for _before in &group.before {}

                let res = route.handler.handle_request(req).map(full);

                for _after in &group.after {}
                for _after in &router.after {}

                Ok(res)
            }),
            None => Box::pin(async move { Ok(not_found()) }),
        };

        for _finish in &self.finish {}

        return fut;
    }
}

fn ensure_max_body_size(req: Request<Incoming>) -> Option<Request<Incoming>> {
    const MAX_BODY_SIZE: u64 = 1024 * 64;

    let upper = req.body().size_hint().upper().unwrap_or(u64::MAX);
    (upper <= MAX_BODY_SIZE).then_some(req)
}

async fn collect_full_request_body(req: Request<Incoming>) -> Result<Request<Bytes>, RouterError> {
    let (parts, body) = req.into_parts();
    let body = body
        .collect()
        .await
        .map_err(|_| RouterError::InvalidBody)?
        .to_bytes();
    Ok(Request::from_parts(parts, body))
}

fn not_found() -> Response<BoxBody<Bytes, hyper::Error>> {
    Response::builder()
        .status(404)
        .body(full("Not Found".into()))
        .unwrap()
}

fn full(bytes: Bytes) -> BoxBody<Bytes, hyper::Error> {
    Full::new(bytes).map_err(|never| match never {}).boxed()
}
