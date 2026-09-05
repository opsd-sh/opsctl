use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const BUSINESS_ID: &str = "22222222-2222-4222-8222-222222222222";

#[test]
fn billing_status_prints_json_and_propagates_api_errors() {
    for (status, body) in [
        (200, r#"{"payment_method_saved":true}"#),
        (200, r#"{"payment_method_saved":false}"#),
        (
            403,
            r#"{"type":"https://api.opsd.sh/problems/forbidden","title":"Forbidden","status":403,"detail":"business admin role required","category":"request"}"#,
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base_url = format!("http://{}/", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "CLI did not send a request");
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("accept failed: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                let count = stream.read(&mut buffer).unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&buffer[..count]);
            }
            write!(stream, "HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            String::from_utf8(request).unwrap()
        });
        let config = tempfile::tempdir().unwrap();
        let path = config.path().join("credentials.json");
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;
        std::fs::write(
            &path,
            serde_json::to_vec(&serde_json::json!({
                "server_url": base_url, "access_token": "test-token", "expires_at": expires_at,
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
            .args([
                "--base-url",
                &base_url,
                "businesses",
                "billing",
                "status",
                BUSINESS_ID,
            ])
            .output()
            .unwrap();
        let request = server.join().unwrap();
        assert_eq!(
            request.lines().next(),
            Some(format!("GET /v1/businesses/{BUSINESS_ID}/billing/status HTTP/1.1").as_str())
        );
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-token\r\n")
        );
        if status == 200 {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
                serde_json::from_str::<serde_json::Value>(body).unwrap()
            );
        } else {
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("business admin role required")
            );
        }
    }
}

#[test]
fn billing_status_requires_login_and_a_valid_business_id() {
    let config = tempfile::tempdir().unwrap();
    for id in [None, Some("invalid"), Some(BUSINESS_ID)] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_opsctl"));
        command
            .env("OPSCTL_CONFIG_DIR", config.path())
            .args(["businesses", "billing", "status"]);
        if let Some(id) = id {
            command.arg(id);
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        if id == Some(BUSINESS_ID) {
            assert!(error.contains("login"), "{error}");
        } else {
            assert_eq!(output.status.code(), Some(2));
        }
    }
}
