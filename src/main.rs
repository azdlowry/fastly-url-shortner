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
        &Method::PUT => {
            return handle_put(req);
        }
        
        // Block requests with unexpected methods
        &Method::POST | &Method::PATCH | &Method::DELETE => {
            return Ok(Response::from_status(StatusCode::METHOD_NOT_ALLOWED)
                .with_header(header::ALLOW, "GET, HEAD, PURGE")
                .with_body_text_plain("This method is not allowed\n"))
        }

        // Let any other requests through
        _ => (),
    };

    handle_get(req)
}

fn handle_put(mut req: Request) -> Result<Response, Error> {
    // Ensure the request has a body
    if !req.has_body() {
        return Ok(Response::from_status(StatusCode::BAD_REQUEST)
            .with_body_text_plain("PUT requests must have a body\n"));
    }

    let body = req.take_body_json::<AddRedirectRequest>()?;

    // Extract the key from the path
    let key = match req.get_path() {
        path if path.len() == 7 && path.chars().skip(1).all(|c| c.is_ascii_alphanumeric()) => &path[1..7],
        _ => return Ok(Response::from_status(StatusCode::BAD_REQUEST)
            .with_body_text_plain("Invalid key format. Use /<6 alphanumeric characters>\n")),
    };
    let path = body.path;

    // Open the KV store and insert the value
    let store = fastly::kv_store::KVStore::open("shortner")?.expect("KVStore not found");
    store.insert(key, path.clone())?;

    log::info!("Stored {} for key {}", path, key);

    Ok(Response::from_status(StatusCode::CREATED)
        .with_content_type(mime::TEXT_PLAIN_UTF_8)
        .with_body_text_plain(&format!("Stored value for key: {}\n", key)))
}

fn handle_get(req: Request) -> Result<Response, Error> {
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

// serde type for add redirect request
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct AddRedirectRequest {
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastly::mime::TEXT_PLAIN_UTF_8;

    #[test]
    fn test_redirect() {
        test_any_redirect("abc123", "https://rustmanchester.co.uk/");
    }

    fn test_any_redirect(key: &'static str, path: &'static str) {
        let req = fastly::Request::get(format!("http://example.com/{}", key));
        let resp = handler(req).expect("request succeeds");
        assert_eq!(resp.get_status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(
            resp.get_header(header::LOCATION).map(|h| h.to_str().ok()),
            Some(Some(path))
        );
        assert_eq!(resp.get_content_type(), Some(TEXT_PLAIN_UTF_8));
        assert_eq!(resp.into_body_str(), "You have been redirected.\n");
    }
    
    #[test]
    fn test_handle_put() {
        let req = fastly::Request::put("http://example.com/xyz789")
            .with_body_json(&AddRedirectRequest {
                path: "https://example.com/xyz789".to_string(),
            })
            .expect("valid JSON body");
        let resp = handler(req).expect("request succeeds");
        assert_eq!(resp.get_status(), StatusCode::CREATED);
        assert_eq!(
            resp.into_body_str(),
            "Stored value for key: xyz789\n"
        );

        test_any_redirect("xyz789", "https://example.com/xyz789");
    }
}
