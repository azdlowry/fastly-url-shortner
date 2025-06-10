//! Default Compute template program.

use fastly::http::header::CACHE_CONTROL;
use fastly::http::{header, Method, StatusCode};
use fastly::{mime, Error, Request, Response};
use log;

/// The entry point for your application.
///
/// This function is triggered when your service receives a client request. It could be used to
/// route based on the request properties (such as method or path), send the request to a backend,
/// make completely new requests, and/or generate synthetic responses.
///
/// If `main` returns an error, a 500 error response will be delivered to the client.
#[fastly::main]
fn main(req: Request) -> Result<Response, Error> {
    // Log service version
    println!(
        "FASTLY_SERVICE_VERSION: {}",
        std::env::var("FASTLY_SERVICE_VERSION").unwrap_or_else(|_| String::new())
    );

    log_fastly::init_simple("my_endpoint", log::LevelFilter::Warn);

    handler(req)
}

fn handler(req: Request) -> Result<Response, Error> {
    // Filter request methods...
    match req.get_method() {
        // Block requests with unexpected methods
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE => {
            return Ok(Response::from_status(StatusCode::METHOD_NOT_ALLOWED)
                .with_header(header::ALLOW, "GET, HEAD, PURGE")
                .with_body_text_plain("This method is not allowed\n"))
        }

        // Let any other requests through
        _ => (),
    };

    // Pattern match on the path...
    match req.get_path() {
        "/" => Ok(Response::from_status(StatusCode::OK)
            .with_content_type(mime::TEXT_HTML_UTF_8)
            .with_body(include_str!("welcome-to-compute.html"))),

        path if path.len() == 7 && path.chars().skip(1).all(|c| c.is_ascii_alphanumeric()) => {
            let key = &path[1..7];
            redirect(key)
        }

        // Catch all other requests and return a 404.
        _ => Ok(Response::from_status(StatusCode::NOT_FOUND)
            .with_body_text_plain("The page you requested could not be found\n")),
    }
}

fn redirect(key: &str) -> Result<Response, Error> {
    let store = fastly::kv_store::KVStore::open("shortner")?.expect("KVStore not found");

    let mut response = store.lookup(key)?;

    let path = response.take_body().into_string();

    log::info!("Redirecting to {} for key {}", path, key);

    Ok(Response::from_status(StatusCode::PERMANENT_REDIRECT)
        .with_header(header::LOCATION, path)
        .with_header(
            CACHE_CONTROL,
            "no-cache, no-store, max-age=0, must-revalidate",
        )
        .with_header(header::PRAGMA, "no-cache")
        .with_header(header::EXPIRES, "0")
        .with_body_text_plain("You have been redirected.\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastly::mime::TEXT_PLAIN_UTF_8;

    #[test]
    fn test_redirect() {
        let req = fastly::Request::get("http://example.com/abc123");
        let resp = handler(req).expect("request succeeds");
        assert_eq!(resp.get_status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(
            resp.get_header(header::LOCATION).map(|h| h.to_str().ok()),
            Some(Some("https://rustmanchester.co.uk/"))
        );
        assert_eq!(resp.get_content_type(), Some(TEXT_PLAIN_UTF_8));
        assert_eq!(resp.into_body_str(), "You have been redirected.\n");
    }
}
