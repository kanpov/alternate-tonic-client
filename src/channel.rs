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

#[derive(Clone)]
pub struct GrpcChannel {
    inner: BoxCloneSyncService<Request<Body>, Response<Incoming>, Box<dyn std::error::Error + Send + Sync>>,
}

impl GrpcChannel {
    #[cfg(feature = "singleton-channel")]
    pub fn singleton(connector: GrpcConnector, buffer_size: usize) -> Self {
        let buffer = tower::buffer::Buffer::new(
            tower::reconnect::Reconnect::new(crate::util::SingletonConnectService::new(connector), ()),
            buffer_size,
        );

        Self {
            inner: BoxCloneSyncService::new(buffer),
        }
    }

    #[cfg(feature = "pooled-channel")]
    pub fn pooled(connector: GrpcConnector) -> Self {
        let mut client_builder = Client::builder(TokioExecutor::new());
        client_builder.http2_only(true);

        let service = client_builder
            .build(connector)
            .map_request(|mut request: Request<Body>| {
                crate::util::set_request_uri_scheme_and_authority(&mut request);
                request
            })
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>);

        GrpcChannel {
            inner: BoxCloneSyncService::new(service),
        }
    }
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
