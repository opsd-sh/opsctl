//! Optional request-line export for opserver's cross-repository coverage check.
//! Only method/path are recorded; credentials and request bodies never are.
use std::io::Write;

pub fn record(request: &str) {
    let Some(directory) = std::env::var_os("OPSCTL_TEST_COVERAGE_DIR") else {
        return;
    };
    let mut parts = request.lines().next().unwrap().split_whitespace();
    let method = parts.next().unwrap();
    let path = parts.next().unwrap().split('?').next().unwrap();
    // A separate file per request avoids races between tests and test binaries.
    let mut file = tempfile::NamedTempFile::new_in(directory).unwrap();
    writeln!(file, "{method} {path}").unwrap();
    file.keep().unwrap();
}
