//! Credentials are supplied for each action, never saved in CLI configuration.
use clap::Args;
use opsd::types::HmrcPassword;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Debug, Args)]
pub(crate) struct HmrcCredentials {
    #[arg(long)]
    pub sender_id: String,
    /// File containing the HMRC password, or - to read standard input.
    /// One trailing newline is removed; other whitespace is preserved.
    #[arg(long)]
    pub password_file: PathBuf,
}
pub(crate) fn read_password(path: &Path) -> Result<HmrcPassword, Box<dyn std::error::Error>> {
    let mut value = String::new();
    if path == Path::new("-") {
        std::io::stdin().read_to_string(&mut value)?;
    } else {
        value = std::fs::read_to_string(path)?;
    }
    if value.ends_with('\n') {
        value.pop();
        if value.ends_with('\r') {
            value.pop();
        }
    }
    if value.is_empty() {
        return Err("HMRC password must not be empty".into());
    }
    Ok(HmrcPassword::new(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn password_file_preserves_spaces_and_removes_one_line_ending() {
        let file = tempfile::NamedTempFile::new().unwrap();
        for (input, expected) in [
            (" secret ", " secret "),
            (" secret \n", " secret "),
            ("secret\r\n", "secret"),
            ("secret\n\n", "secret\n"),
        ] {
            std::fs::write(file.path(), input).unwrap();
            let password = read_password(file.path()).unwrap();
            assert_eq!(serde_json::to_value(&password).unwrap(), expected);
            assert!(!format!("{password:?}").contains("secret"));
        }
        std::fs::write(file.path(), "\n").unwrap();
        assert!(read_password(file.path()).is_err());
    }
}
