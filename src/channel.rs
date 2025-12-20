use std::{
    pin::Pin,
    task::{Context, Poll},
};

use http::{Request, Response};
use hyper::body::Incoming;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use tonic::body::Body;
use tower::{Service, ServiceExt, util::BoxCloneSyncService};

use crate::connector::GrpcConnector;

pub struct GrpcChannelBuilder {}

impl GrpcChannelBuilder {
    pub fn new() -> Self {
        Self {}
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
        let client = Client::builder(TokioExecutor::new())
            .build::<_, Body>(connector)
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>);

        GrpcChannel {
            inner: BoxCloneSyncService::new(client),
        }
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
