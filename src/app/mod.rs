use std::net::SocketAddr;

use http::Method;

use crate::{
    router::{
        groups::Group,
        routes::{Handler, Route},
        Router,
    },
    server::Server,
};

pub struct App {
    router: Router,
}

impl App {
    pub fn new() -> Self {
        Self {
            router: Router::new(),
        }
    }

    pub fn mount<F>(&mut self, prefix: &str, func: F) -> &mut App
    where
        F: Fn(&mut Group),
    {
        let mut group = Group::new(prefix);

        func(&mut group);

        self.router.groups.push(group);
        self
    }

    pub fn mount_group(&mut self, group: Group) -> &mut App {
        self.router.groups.push(group);
        self
    }

    pub fn add<H>(&mut self, method: Method, pattern: &str, handle: H) -> &mut Route
    where
        H: Handler,
    {
        self.router
            .groups
            .get_mut(0)
            .unwrap()
            .add(method, pattern, handle)
    }

    pub fn get<H>(&mut self, pattern: &str, handle: H) -> &mut Route
    where
        H: Handler,
    {
        self.add(Method::GET, pattern, handle)
    }

    pub fn run<A: Into<SocketAddr>>(self, addr: A) -> std::io::Result<()> {
        let server = Server::new(addr.into(), self.router);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(server.bind())
    }
}
