use std::{
    pin::Pin,
    task::{Context, Poll},
};

use http::Uri;
use tower::Service;

use crate::stream::GrpcStream;

#[cfg(feature = "custom-transport")]
pub type CustomGrpcConnector = tower::util::BoxCloneSyncService<
    (),
    Box<dyn crate::stream::CustomGrpcStream>,
    Box<dyn std::error::Error + Send + Sync>,
>;

#[derive(Clone)]
pub enum GrpcConnector {
    #[cfg(feature = "unix-transport")]
    Unix(std::sync::Arc<std::path::PathBuf>),
    #[cfg(feature = "custom-transport")]
    Custom(CustomGrpcConnector),
}

impl Service<Uri> for GrpcConnector {
    type Response = GrpcStream;

    type Error = Box<dyn std::error::Error + Send + Sync>;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        #[cfg_attr(feature = "unix-transport", allow(unused))] cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        match self {
            #[cfg(feature = "unix-transport")]
            GrpcConnector::Unix(_) => Poll::Ready(Ok(())),
            #[cfg(feature = "custom-transport")]
            GrpcConnector::Custom(connector) => connector.poll_ready(cx),
        }
    }

    fn call(&mut self, _uri: Uri) -> Self::Future {
        match self {
            #[cfg(feature = "unix-transport")]
            GrpcConnector::Unix(socket_path) => {
                let socket_path = socket_path.clone();

                Box::pin(async move {
                    let stream = tokio::net::UnixStream::connect(socket_path.as_ref()).await?;
                    Ok(GrpcStream::Unix(hyper_util::rt::TokioIo::new(stream)))
                })
            }
            GrpcConnector::Custom(connector) => {
                let mut connector = connector.clone();

                Box::pin(async move {
                    let stream = connector.call(()).await?;
                    Ok(GrpcStream::Custom(stream))
                })
            }
        }
    }
}
