use asteria::net::http::HttpClient;
use asteria::net::bus::ResourceBusEvent;
use std::sync::mpsc;
use std::thread;

#[test]
fn test_fetch_wikipedia_integration() {
    let mut client = HttpClient::new();
    let url = "https://en.wikipedia.org/wiki/Main_Page";
    let (sender, receiver) = mpsc::channel();
    
    // Spawn network fetch on another thread
    thread::spawn(move || {
        let result = client.stream(url, sender);
        assert!(result.is_ok(), "Failed to stream wikipedia");
    });

    let mut total_bytes = 0;
    let mut headers_received = false;
    let mut finished = false;

    for event in receiver {
        match event {
            ResourceBusEvent::HeaderReceived { status_code, .. } => {
                assert_eq!(status_code, 200, "Expected status 200 OK");
                headers_received = true;
            }
            ResourceBusEvent::ChunkReceived { chunk_data, is_final, .. } => {
                total_bytes += chunk_data.len();
                if is_final {
                    finished = true;
                }
            }
            ResourceBusEvent::FetchError { message, .. } => {
                panic!("Network error occurred: {}", message);
            }
        }
    }

    assert!(headers_received, "Did not receive headers from Wikipedia");
    assert!(finished, "Stream did not finish properly");
    assert!(total_bytes > 10000, "Expected a substantial body from Wikipedia, got {} bytes", total_bytes);
}
