use http::Request;
use tonic::body::Body;

pub fn set_request_uri_scheme_and_authority(request: &mut Request<Body>) {
    *request.uri_mut() = http::Uri::builder()
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
}

#[cfg(feature = "singleton-channel")]
pub struct SingletonConnectService {
    connector: crate::GrpcConnector,
}

#[cfg(feature = "singleton-channel")]
impl SingletonConnectService {
    pub fn new(connector: crate::GrpcConnector) -> Self {
        Self { connector }
    }
}

#[cfg(feature = "singleton-channel")]
impl tower::Service<()> for SingletonConnectService {
    type Response = crate::util::SingletonService;

    type Error = Box<dyn std::error::Error + Send + Sync>;

    type Future = std::pin::Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.connector.poll_ready(cx)
    }

    fn call(&mut self, _: ()) -> Self::Future {
        let mut connector = self.connector.clone();

        Box::pin(async move {
            let stream = connector.call(http::Uri::from_static("http://localhost")).await?;
            let (send_request, connection) =
                hyper::client::conn::http2::handshake::<_, _, Body>(hyper_util::rt::TokioExecutor::new(), stream)
                    .await?;

            tokio::task::spawn(connection);

            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(SingletonService { send_request })
        })
    }
}

#[cfg(feature = "singleton-channel")]
#[derive(Clone)]
pub struct SingletonService {
    send_request: hyper::client::conn::http2::SendRequest<Body>,
}

#[cfg(feature = "singleton-channel")]
impl tower::Service<Request<Body>> for SingletonService {
    type Response = http::Response<hyper::body::Incoming>;

    type Error = Box<dyn std::error::Error + Send + Sync>;

    type Future = std::pin::Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.send_request
            .poll_ready(cx)
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)
    }

    fn call(&mut self, mut request: Request<Body>) -> Self::Future {
        set_request_uri_scheme_and_authority(&mut request);
        let future = self.send_request.send_request(request);

        Box::pin(async {
            future
                .await
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)
        })
    }
}
