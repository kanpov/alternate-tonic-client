use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use http::{Request, Response, Uri};
use hyper::body::Incoming;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use tonic::body::Body;
use tower::{Service, ServiceExt, timeout::Timeout, util::BoxCloneSyncService};

use crate::connector::GrpcConnector;

pub struct GrpcChannelBuilder {
    connection_timeout: Option<Duration>,
    request_timeout: Option<Duration>,
}

impl GrpcChannelBuilder {
    pub fn new() -> Self {
        Self {
            connection_timeout: None,
            request_timeout: None,
        }
    }

    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = Some(timeout);
        self
    }

    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    #[cfg(feature = "unix-transport")]
    pub fn build_with_unix_transport<P: Into<std::path::PathBuf>>(self, socket_path: P) -> GrpcChannel {
        self.build(GrpcConnector::Unix(std::sync::Arc::new(socket_path.into())))
    }

    #[cfg(feature = "custom-transport")]
    pub fn build_with_custom_hyper_transport<C>(self, connector: C) -> GrpcChannel
    where
        C: tower::Service<()> + Clone + Send + Sync + 'static,
        C::Error: std::error::Error + Send + Sync,
        C::Response: hyper::rt::Read + hyper::rt::Write + Send + Unpin,
        C::Future: Future<Output = Result<C::Response, C::Error>> + Send,
    {
        let connector = GrpcConnector::Custom(BoxCloneSyncService::new(
            connector
                .map_response(|stream| Box::new(stream) as Box<dyn crate::stream::CustomGrpcStream>)
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>),
        ));

        self.build(connector)
    }

    #[cfg(feature = "custom-transport")]
    pub fn build_with_custom_tokio_transport<C>(self, connector: C) -> GrpcChannel
    where
        C: tower::Service<()> + Clone + Send + Sync + 'static,
        C::Error: std::error::Error + Send + Sync,
        C::Response: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin,
        C::Future: Future<Output = Result<C::Response, C::Error>> + Send,
    {
        self.build_with_custom_hyper_transport(connector.map_response(|stream| hyper_util::rt::TokioIo::new(stream)))
    }

    fn build(self, connector: GrpcConnector) -> GrpcChannel {
        let connector = match self.connection_timeout {
            Some(timeout) => BoxCloneSyncService::new(Timeout::new(connector, timeout)),
            None => BoxCloneSyncService::new(connector),
        };

        let mut client_builder = Client::builder(TokioExecutor::new());
        client_builder.http2_only(true);

        let client = client_builder
            .build(connector)
            .map_request(|mut request: Request<Body>| {
                *request.uri_mut() = Uri::builder()
                    .scheme("http")
                    .authority("localhost")
                    .path_and_query(
                        request
                            .uri()
                            .path_and_query()
                            .expect("No path and query were specified for a gRPC request")
                            .clone(),
                    )
                    .build()
                    .expect("Uri builder failed");

                request
            })
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>);

        let client = match self.request_timeout {
            Some(timeout) => BoxCloneSyncService::new(Timeout::new(client, timeout)),
            None => BoxCloneSyncService::new(client),
        };

        GrpcChannel { inner: client }
    }
}

#[derive(Debug, Clone)]
pub struct GrpcChannel {
    inner: BoxCloneSyncService<Request<Body>, Response<Incoming>, Box<dyn std::error::Error + Send + Sync>>,
}

impl Service<Request<Body>> for GrpcChannel {
    type Response = Response<Incoming>;

    type Error = Box<dyn std::error::Error + Send + Sync>;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        self.inner.call(request)
    }
}
