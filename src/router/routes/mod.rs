mod handler;
mod paths;

use std::fmt;

use http::Method;

use super::middleware::BoxedMiddleware;

pub use self::{
    handler::{FromRequest, Handler, IntoResponse},
    params::Param,
    paths::{Match, Path},
};

pub struct Route {
    pub(super) path: Path,
    method: Method,
    pub(crate) handler: Box<dyn Handler>,
    pub(crate) before: Vec<BoxedMiddleware>,
    pub(crate) after: Vec<BoxedMiddleware>,
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.method.as_str(), self.path.as_ref())
    }
}

impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)?;
        write!(
            f,
            " {{ handler: ..., before: [...; {}], after: [...; {}] }}",
            self.before.len(),
            self.after.len()
        )
    }
}

impl Route {
    pub fn new(method: Method, path: Path, handler: Box<dyn Handler>) -> Self {
        Self {
            path,
            method,
            handler,
            before: Vec::new(),
            after: Vec::new(),
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }
}

mod params {
    use std::ops::{Deref, DerefMut};

    pub struct Param<T>(T);

    impl<T> Param<T> {
        pub const fn new(t: T) -> Self {
            Self(t)
        }

        pub fn into_inner(self) -> T {
            self.0
        }
    }

    impl<T> Deref for Param<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T> DerefMut for Param<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}
