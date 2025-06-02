use bytes::Bytes;
use http_body_util::Full;
use hyper::Request;
use hyper::Response;
use hyper::body::Incoming as IncomingBody;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as HyperAutoBuilder;
use std::net::SocketAddr;

#[derive(Clone)]
struct MainRouterService {}

impl tower::Service<Request<IncomingBody>> for MainRouterService {
    type Response = Response<Full<Bytes>>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = futures_util::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<IncomingBody>) -> Self::Future {
        let path = req.uri().path().to_string();
        Box::pin(async move {
            if path == "/hello" {
                Ok(Response::new(Full::from("Hello, Hyper+Tower!")))
            } else {
                let mut not_found = Response::new(Full::from("Not Found"));
                *not_found.status_mut() = hyper::StatusCode::NOT_FOUND;
                Ok(not_found)
            }
        })
    }
}

fn is_connection_error(err: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
    let err_str = err.to_string().to_lowercase();
    err_str.contains("connection reset by peer")
        || err_str.contains("broken pipe")
        || err_str.contains("connection aborted")
        || err_str.contains("unexpected eof")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let settings_listen_address = "127.0.0.1:3000".parse::<SocketAddr>()?;
    let main_router_service = MainRouterService {};
    let listener = tokio::net::TcpListener::bind(settings_listen_address).await?;
    println!("Listening on http://{}", settings_listen_address);
    let executor = TokioExecutor::new();
    loop {
        let (tcp_stream, remote_addr) = listener.accept().await?;
        let io = TokioIo::new(tcp_stream);
        let service_clone = main_router_service.clone();
        let hyper_service = hyper_util::service::TowerToHyperService::new(service_clone);
        let connection_executor = executor.clone();
        tokio::task::spawn(async move {
            let builder = HyperAutoBuilder::new(connection_executor);
            if let Err(err) = builder.serve_connection(io, hyper_service).await {
                if !is_connection_error(&*err) {
                    eprintln!("Error serving connection from {}: {:?}", remote_addr, err);
                }
            }
        });
    }
}
