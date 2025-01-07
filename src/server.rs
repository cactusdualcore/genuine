use std::{io, net::SocketAddr, sync::Arc};

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::router::Router;

pub struct Server {
    addr: SocketAddr,
    router: Router,
}

impl Server {
    pub fn new(addr: SocketAddr, router: Router) -> Self {
        Self { addr, router }
    }

    pub async fn bind(self) -> io::Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        let router = Arc::new(self.router);

        loop {
            let (stream, _) = listener.accept().await?;
            let router_handle = Arc::clone(&router);

            // Use an adapter to access something implementing `tokio::io` traits as if they implement
            // `hyper::rt` IO traits.
            let io = TokioIo::new(stream);

            // Spawn a tokio task to serve multiple connections concurrently
            tokio::task::spawn(async move {
                // Finally, we bind the incoming connection to our `hello` service
                if let Err(err) = http1::Builder::new()
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(io, router_handle)
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}
