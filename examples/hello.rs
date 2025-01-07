use genuine::App;
use http::{header::CONTENT_TYPE, Response};
use hyper::body::Bytes;

fn main() {
    let mut app = App::new();

    let greet = || {
        Response::builder()
            .header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Bytes::from("Hello, world!"))
            .unwrap()
    };

    app.get("/", greet as fn() -> _);

    app.run(([127, 0, 0, 1], 3000)).unwrap();
}
