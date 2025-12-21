#[cfg(feature = "singleton-channel")]
#[derive(Clone)]
pub struct SendRequestService {
    pub send_request: hyper::client::conn::http2::SendRequest<tonic::body::Body>,
}

#[cfg(feature = "singleton-channel")]
impl tower::Service<http::Request<tonic::body::Body>> for SendRequestService {
    type Response = http::Response<hyper::body::Incoming>;

    type Error = Box<dyn std::error::Error + Send + Sync>;

    type Future = std::pin::Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.send_request
            .poll_ready(cx)
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)
    }

    fn call(&mut self, mut request: http::Request<tonic::body::Body>) -> Self::Future {
        set_request_uri_scheme_and_authority(&mut request);
        let future = self.send_request.send_request(request);

        Box::pin(async {
            future
                .await
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)
        })
    }
}

#[cfg(feature = "_channel")]
pub fn set_request_uri_scheme_and_authority(request: &mut http::Request<tonic::body::Body>) {
    use http::Uri;

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
}
