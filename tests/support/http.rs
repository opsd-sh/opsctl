//! Local HTTP server and CLI request/response assertions.
use super::coverage;

use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Output},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub fn request(args: &[&str], status: u16, content_type: &str, body: &[u8]) -> (Output, String) {
    let (output, mut requests) =
        requests(args, vec![(status, content_type.to_owned(), body.to_vec())]);
    (output, requests.remove(0))
}
pub fn requests(args: &[&str], responses: Vec<(u16, String, Vec<u8>)>) -> (Output, Vec<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base_url = format!("http://{}/", listener.local_addr().unwrap());

    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, content_type, body) in responses {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "CLI did not send a request");
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|b| b == b"\r\n\r\n") {
                    let length = String::from_utf8_lossy(&request[..end])
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .map(|v| v.parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            write!(stream,"HTTP/1.1 {status} Result\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();
            stream.write_all(&body).unwrap();
            requests.push(String::from_utf8(request).unwrap());
        }
        requests
    });
    let config = tempfile::tempdir().unwrap();
    let path = config.path().join("credentials.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({
            "server_url":base_url,"access_token":"test-token",
            "expires_at":SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()+3600,
        }))
        .unwrap(),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_opsctl"))
        .env("OPSCTL_CONFIG_DIR", config.path())
        .args(["--base-url", &base_url])
        .args(args)
        .output()
        .unwrap();
    let request = server.join().unwrap();
    for request in &request {
        coverage::record(request);
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-token\r\n")
        );
    }
    (output, request)
}

pub fn check(
    args: &[&str],
    method: &str,
    path: &str,
    sent: Option<Value>,
    response: Option<Value>,
) {
    let body = response.as_ref().map(Value::to_string).unwrap_or_default();
    let (output, request) = request(
        args,
        if response.is_some() { 200 } else { 204 },
        "application/json",
        body.as_bytes(),
    );
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        request.lines().next().unwrap(),
        format!("{method} /v1/{path} HTTP/1.1")
    );
    let actual_body = request.split_once("\r\n\r\n").unwrap().1;
    if let Some(sent) = sent {
        assert_eq!(serde_json::from_str::<Value>(actual_body).unwrap(), sent);
    } else {
        assert!(actual_body.is_empty());
    }
    if let Some(response) = response {
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            response
        );
    } else {
        assert!(output.stdout.is_empty());
    }
}
