pub trait Middleware {}

pub type BoxedMiddleware = Box<dyn Middleware + Send + Sync + 'static>;
