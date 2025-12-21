use std::{
    pin::Pin,
    task::{Context, Poll},
};

use http::{Request, Response, Uri};
use hyper::body::Incoming;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use tonic::body::Body;
use tower::{Service, ServiceExt, util::BoxCloneSyncService};

use crate::connector::GrpcConnector;

pub struct GrpcConnectionPoolOptions {}

#[derive(Debug, Clone)]
pub struct GrpcChannel {
    inner: BoxCloneSyncService<Request<Body>, Response<Incoming>, Box<dyn std::error::Error + Send + Sync>>,
}

impl GrpcChannel {
    #[cfg(feature = "singleton-channel")]
    pub async fn singleton(mut connector: GrpcConnector) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let connector = connector.ready().await?;
        let stream = connector.call(Uri::from_static("http://localhost")).await?;
        let (send_request, connection) =
            hyper::client::conn::http2::handshake::<_, _, Body>(TokioExecutor::new(), stream).await?;

        tokio::task::spawn(connection);

        Ok(Self {
            inner: BoxCloneSyncService::new(crate::util::SendRequestService { send_request }),
        })
    }

    #[cfg(feature = "pooled-channel")]
    pub fn pooled(connector: GrpcConnector) -> Self {
        let mut client_builder = Client::builder(TokioExecutor::new());
        client_builder.http2_only(true);

        let client = client_builder
            .build(connector)
            .map_request(|mut request: Request<Body>| {
                crate::util::set_request_uri_scheme_and_authority(&mut request);
                request
            })
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>);

        GrpcChannel {
            inner: BoxCloneSyncService::new(client),
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
