/// Route group
use std::collections::HashMap;

use http::Method;

use super::middleware::BoxedMiddleware;
use super::routes::{Handler, Path, Route};

// use hyper::Method;

/// Route group
///
/// # Examples
///
/// ```
/// use sincere::App;
/// use sincere::app::Group;
///
/// let mut group = Group::new("/app");
///
/// group.get("/", |context| {
///     context.response.from_text("Hello world!").unwrap();
/// });
///
/// group.post("/", |context| {
///     context.response.from_text("Hello world!").unwrap();
/// });
///
/// let mut app = App::new();
///
/// app.mount_group(group);
/// ```
/// or
///
/// ```
/// use sincere::App;
///
/// let mut app = App::new();
///
/// app.mount("/app", |group| {
///
///     group.get("/", |context| {
///         context.response.from_text("Get method!").unwrap();
///     });
///
///     group.post("/", |context| {
///         context.response.from_text("Post method!").unwrap();
///     });
///
/// });
/// ```
pub struct Group {
    pub routes: HashMap<Method, Vec<Route>>,
    prefix: String,
    pub before: Vec<BoxedMiddleware>,
    pub after: Vec<BoxedMiddleware>,
}

impl Group {
    /// Create a route group.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::app::Group;
    ///
    /// let group = Group::new("/app");
    /// ```
    ///
    pub fn new(prefix: &str) -> Group {
        Group {
            routes: HashMap::new(),
            prefix: prefix.to_owned(),
            before: Vec::new(),
            after: Vec::new(),
        }
    }

    /// Add route handle to group.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::app::Group;
    /// use sincere::http::Method;
    ///
    /// let mut group = Group::new("/app");
    ///
    /// group.add(Method::GET, "/", |context| {
    ///     context.response.from_text("Get method!").unwrap();
    /// });
    /// ```
    pub fn add<H>(&mut self, method: Method, pattern: &str, handler: H) -> &mut Route
    where
        H: Handler,
    {
        let path = Path::new(self.prefix.clone() + pattern).unwrap();
        let route = Route::new(method.clone(), path, Box::new(handler));

        let routes = self.routes.entry(method).or_default();
        routes.push(route);
        routes.last_mut().unwrap()
    }
}
