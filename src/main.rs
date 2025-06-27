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
    // Special handling for API endpoints that only accept GET
    if req.get_path() == "/api/shortcodes" {
        match req.get_method() {
            &Method::GET | &Method::HEAD | &Method::OPTIONS => list_shortcodes(),
            _ => Ok(Response::from_status(StatusCode::METHOD_NOT_ALLOWED)
                .with_header(header::ALLOW, "GET")
                .with_body_text_plain("Only GET method is allowed for this endpoint\n")),
        }
    } else if req.get_path().starts_with("/api/shortcodes/") {
        match req.get_method() {
            &Method::GET | &Method::HEAD | &Method::OPTIONS => handle_api_get(req),
            &Method::PUT => handle_put(req),
            &Method::DELETE => handle_api_delete(req),
            _ => Ok(Response::from_status(StatusCode::METHOD_NOT_ALLOWED)
                .with_header(header::ALLOW, "GET")
                .with_body_text_plain("Only GET method is allowed for this endpoint\n")),
        }
    } else {
        // Filter request methods...
        match req.get_method() {
            &Method::GET | &Method::HEAD | &Method::OPTIONS => handle_get(req),

            // Block requests with unexpected methods
            _ => {
                return Ok(Response::from_status(StatusCode::METHOD_NOT_ALLOWED)
                    .with_header(header::ALLOW, "GET, HEAD, OPTIONS")
                    .with_body_text_plain("This method is not allowed\n"))
            }
        }
    }
}

fn handle_put(mut req: Request) -> Result<Response, Error> {
    // Ensure the request has a body
    if !req.has_body() {
        return Ok(Response::from_status(StatusCode::BAD_REQUEST)
            .with_body_text_plain("PUT requests must have a body\n"));
    }

    let body = req.take_body_json::<AddRedirectRequest>()?;

    // Extract the key from the path - expect /api/shortcodes/{shortcode}
    let key = match req.get_path() {
        path if path.starts_with("/api/shortcodes/") && path.len() == 22 => {
            let shortcode = &path[16..22]; // Extract 6 chars after "/api/shortcodes/"
            if shortcode.chars().all(|c| c.is_ascii_alphanumeric()) {
                shortcode
            } else {
                return Ok(
                    Response::from_status(StatusCode::BAD_REQUEST).with_body_text_plain(
                        "Invalid shortcode format. Use 6 alphanumeric characters\n",
                    ),
                );
            }
        }
        _ => {
            return Ok(
                Response::from_status(StatusCode::BAD_REQUEST).with_body_text_plain(
                    "Invalid path format. Use /api/shortcodes/<6 alphanumeric characters>\n",
                ),
            )
        }
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

fn handle_api_get(req: Request) -> Result<Response, Error> {
    // Extract the key from the path - expect /api/shortcodes/{shortcode}
    let key = match req.get_path() {
        path if path.starts_with("/api/shortcodes/") && path.len() == 22 => {
            let shortcode = &path[16..22];
            if shortcode.chars().all(|c| c.is_ascii_alphanumeric()) {
                shortcode
            } else {
                return Ok(
                    Response::from_status(StatusCode::BAD_REQUEST).with_body_text_plain(
                        "Invalid shortcode format. Use 6 alphanumeric characters\n",
                    ),
                );
            }
        }
        _ => {
            return Ok(
                Response::from_status(StatusCode::BAD_REQUEST).with_body_text_plain(
                    "Invalid path format. Use /api/shortcodes/<6 alphanumeric characters>\n",
                ),
            )
        }
    };

    let store = fastly::kv_store::KVStore::open("shortner")?.expect("KVStore not found");

    let mut response = store.lookup(key)?;

    let path = response.take_body().into_string();

    log::info!("Lookup {} for key {}", path, key);

    let response_data = ShortcodeEntry {
        shortcode: key.to_string(),
        url: path,
    };

    Ok(Response::from_status(StatusCode::OK)
        .with_content_type(mime::APPLICATION_JSON)
        .with_body_json(&response_data)?)
}

fn handle_api_delete(req: Request) -> Result<Response, Error> {
    // Extract the key from the path - expect /api/shortcodes/{shortcode}
    let key = match req.get_path() {
        path if path.starts_with("/api/shortcodes/") && path.len() == 22 => {
            let shortcode = &path[16..22];
            if shortcode.chars().all(|c| c.is_ascii_alphanumeric()) {
                shortcode
            } else {
                return Ok(
                    Response::from_status(StatusCode::BAD_REQUEST).with_body_text_plain(
                        "Invalid shortcode format. Use 6 alphanumeric characters\n",
                    ),
                );
            }
        }
        _ => {
            return Ok(
                Response::from_status(StatusCode::BAD_REQUEST).with_body_text_plain(
                    "Invalid path format. Use /api/shortcodes/<6 alphanumeric characters>\n",
                ),
            )
        }
    };

    // Open the KV store and remove the entry
    let store = fastly::kv_store::KVStore::open("shortner")?.expect("KVStore not found");
    store.delete(key)?;

    log::info!("Deleted shortcode for key {}", key);

    Ok(Response::from_status(StatusCode::NO_CONTENT))
}

fn handle_get(req: Request) -> Result<Response, Error> {
    // Pattern match on the path...
    match req.get_path() {
        "/" => Ok(Response::from_status(StatusCode::OK)
            .with_content_type(mime::TEXT_HTML_UTF_8)
            .with_body(include_str!("index.html"))),

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

    let mut response = match store.lookup(key) {
        Ok(resp) => resp,
        Err(fastly::kv_store::KVStoreError::ItemNotFound) => {
            return Ok(Response::from_status(StatusCode::NOT_FOUND)
                .with_body_text_plain("Shortcode not found\n"));
        }
        Err(e) => {
            log::error!("Error looking up key {}: {}", key, e);
            return Ok(Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
                .with_body_text_plain("Internal server error\n"));
        }
    };

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

// serde type for shortcode list response
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct ShortcodeEntry {
    pub shortcode: String,
    pub url: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct ShortcodeListResponse {
    pub shortcodes: Vec<ShortcodeEntry>,
}

fn list_shortcodes() -> Result<Response, Error> {
    let store = fastly::kv_store::KVStore::open("shortner")?.expect("KVStore not found");

    let mut shortcodes = Vec::new();

    // Get the list of keys from the store
    let list_page = store.list()?;

    // Iterate through all keys in the list page
    for key in list_page.keys() {
        let mut response = store.lookup(key)?;
        let url = response.take_body().into_string();

        // Only add if we got a non-empty value
        if !url.is_empty() {
            shortcodes.push(ShortcodeEntry {
                shortcode: key.to_string(),
                url,
            });
        }
    }

    // Sort by shortcode for consistent ordering
    shortcodes.sort_by(|a, b| a.shortcode.cmp(&b.shortcode));

    let response_data = ShortcodeListResponse { shortcodes };

    Ok(Response::from_status(StatusCode::OK)
        .with_content_type(mime::APPLICATION_JSON)
        .with_body_json(&response_data)?)
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
        let req = fastly::Request::put("http://example.com/api/shortcodes/xyz789")
            .with_body_json(&AddRedirectRequest {
                path: "https://example.com/xyz789".to_string(),
            })
            .expect("valid JSON body");
        let resp = handler(req).expect("request succeeds");
        assert_eq!(resp.get_status(), StatusCode::CREATED);
        assert_eq!(resp.into_body_str(), "Stored value for key: xyz789\n");

        test_any_redirect("xyz789", "https://example.com/xyz789");
    }

    #[test]
    fn test_list_shortcodes_with_data() {
        // First, add some shortcodes
        let shortcodes = vec![
            ("abc123", "https://example.com/page1"),
            ("def456", "https://example.com/page2"),
            ("ghi789", "https://example.com/page3"),
        ];

        for (shortcode, url) in &shortcodes {
            let req =
                fastly::Request::put(&format!("http://example.com/api/shortcodes/{}", shortcode))
                    .with_body_json(&AddRedirectRequest {
                        path: url.to_string(),
                    })
                    .expect("valid JSON body");
            let resp = handler(req).expect("request succeeds");
            assert_eq!(resp.get_status(), StatusCode::CREATED);
        }

        // Now test the list endpoint
        let req = fastly::Request::get("http://example.com/api/shortcodes");
        let resp = handler(req).expect("request succeeds");
        assert_eq!(resp.get_status(), StatusCode::OK);
        assert_eq!(resp.get_content_type(), Some(mime::APPLICATION_JSON));

        let body = resp.into_body_str();
        let response: ShortcodeListResponse =
            serde_json::from_str(&body).expect("response should be valid JSON");

        // Should have all the shortcodes we added
        assert_eq!(response.shortcodes.len(), shortcodes.len());

        // Verify the shortcodes are sorted alphabetically
        let mut expected_codes: Vec<_> = shortcodes.iter().map(|(code, _)| *code).collect();
        expected_codes.sort();

        for (i, expected_code) in expected_codes.iter().enumerate() {
            assert_eq!(response.shortcodes[i].shortcode, *expected_code);

            // Find the corresponding URL
            let expected_url = shortcodes
                .iter()
                .find(|(code, _)| code == expected_code)
                .map(|(_, url)| *url)
                .expect("should find URL for shortcode");

            assert_eq!(response.shortcodes[i].url, expected_url);
        }
    }

    #[test]
    fn test_list_shortcodes_json_structure() {
        // Add one shortcode
        let req = fastly::Request::put("http://example.com/api/shortcodes/test01")
            .with_body_json(&AddRedirectRequest {
                path: "https://test.example.com".to_string(),
            })
            .expect("valid JSON body");
        let resp = handler(req).expect("request succeeds");
        assert_eq!(resp.get_status(), StatusCode::CREATED);

        // Test the list endpoint
        let req = fastly::Request::get("http://example.com/api/shortcodes");
        let resp = handler(req).expect("request succeeds");
        assert_eq!(resp.get_status(), StatusCode::OK);

        let body = resp.into_body_str();

        // Verify JSON structure
        let json_value: serde_json::Value =
            serde_json::from_str(&body).expect("response should be valid JSON");

        assert!(json_value.is_object());
        assert!(json_value.get("shortcodes").is_some());
        assert!(json_value["shortcodes"].is_array());

        let shortcodes_array = json_value["shortcodes"].as_array().unwrap();
        assert!(shortcodes_array.len() >= 1);

        // Check first shortcode structure
        let first_shortcode = &shortcodes_array[0];
        assert!(first_shortcode.get("shortcode").is_some());
        assert!(first_shortcode.get("url").is_some());
        assert!(first_shortcode["shortcode"].is_string());
        assert!(first_shortcode["url"].is_string());
    }

    #[test]
    fn test_api_shortcodes_endpoint_method_validation() {
        // Test that only GET is allowed on /api/shortcodes
        let methods = vec![Method::POST, Method::PUT, Method::DELETE, Method::PATCH];

        for method in methods {
            let req = fastly::Request::new(&method, "http://example.com/api/shortcodes");
            let resp = handler(req).expect("request succeeds");
            assert_eq!(
                resp.get_status(),
                StatusCode::METHOD_NOT_ALLOWED,
                "Method: {:?}",
                &method
            );
        }
    }

    #[test]
    fn test_handle_delete() {
        // First, create a shortcode
        let req = fastly::Request::put("http://example.com/api/shortcodes/del123")
            .with_body_json(&AddRedirectRequest {
                path: "https://example.com/delete-test".to_string(),
            })
            .expect("valid JSON body");
        let resp = handler(req).expect("request succeeds");
        assert_eq!(resp.get_status(), StatusCode::CREATED);

        // Verify it exists by trying to redirect
        let req = fastly::Request::get("http://example.com/del123");
        let resp = handler(req).expect("request succeeds");
        assert_eq!(resp.get_status(), StatusCode::PERMANENT_REDIRECT);

        // Now delete it
        let req = fastly::Request::delete("http://example.com/api/shortcodes/del123");
        let resp = handler(req).expect("request succeeds");
        assert_eq!(resp.get_status(), StatusCode::NO_CONTENT);

        // Verify it's gone by trying to redirect (should 404)
        let req = fastly::Request::get("http://example.com/del123");
        let resp = handler(req).expect("KV Store item not found");
        assert_eq!(resp.get_status(), StatusCode::NOT_FOUND);
    }
}
