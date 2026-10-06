//! Shared billing pagination and safe PDF output.
use clap::Args;
use opsd::types::{
    BillingMonth, BillingPageSize, ListInvoiceLinesRequest, ListInvoicesRequest,
    ListPaymentMethodsRequest, StripePaymentMethodId,
};
use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

fn page_size(value: &str) -> Result<BillingPageSize, String> {
    let value = value
        .parse::<u16>()
        .map_err(|_| "limit must be between 1 and 100".to_owned())?;
    BillingPageSize::parse(value).map_err(|error| error.to_string())
}

#[derive(Debug, Args)]
pub(crate) struct InvoicePage {
    /// Exclusive billing month cursor, YYYY-MM.
    #[arg(long)]
    before: Option<BillingMonth>,
    /// Page size (1–100; server default 25).
    #[arg(long, value_parser = page_size)]
    limit: Option<BillingPageSize>,
}
impl From<InvoicePage> for ListInvoicesRequest {
    fn from(value: InvoicePage) -> Self {
        Self {
            before: value.before,
            limit: value.limit,
        }
    }
}

#[derive(Debug, Args)]
pub(crate) struct LinePage {
    /// Offset returned as next_offset in the previous page.
    #[arg(long)]
    offset: Option<u32>,
    /// Page size (1–100; server default 25).
    #[arg(long, value_parser = page_size)]
    limit: Option<BillingPageSize>,
}
impl From<LinePage> for ListInvoiceLinesRequest {
    fn from(value: LinePage) -> Self {
        Self {
            offset: value.offset,
            limit: value.limit,
        }
    }
}

#[derive(Debug, Args)]
pub(crate) struct PaymentMethodPage {
    /// Payment method ID returned as next_starting_after in the previous page.
    #[arg(long)]
    starting_after: Option<StripePaymentMethodId>,
    /// Page size (1–100; server default 25).
    #[arg(long, value_parser = page_size)]
    limit: Option<BillingPageSize>,
}
impl From<PaymentMethodPage> for ListPaymentMethodsRequest {
    fn from(value: PaymentMethodPage) -> Self {
        Self {
            starting_after: value.starting_after,
            limit: value.limit,
        }
    }
}

/// Create only after the API download succeeds. Never overwrite existing files.
pub(crate) fn save_pdf(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_new_pdf(path, |file| file.write_all(bytes))?;
    eprintln!("Saved invoice PDF to {}", path.display());
    Ok(())
}

/// Keep incomplete writes out of the destination path. The temporary file is
/// in the same directory and is removed on error; on Unix it is owner-only.
fn write_new_pdf(
    path: &Path,
    write_contents: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    write_contents(temporary.as_file_mut())?;
    temporary.as_file_mut().flush()?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(path)
        .map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{save_pdf, write_new_pdf};
    use std::{
        fs,
        io::{self, Write},
    };

    #[test]
    fn failed_write_cleans_up_and_allows_retry() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("invoice.pdf");
        let error = write_new_pdf(&path, |file| {
            file.write_all(b"%PDF-partial")?;
            assert!(!path.exists(), "incomplete output must not be visible");
            Err(io::Error::other("simulated write failure"))
        })
        .unwrap_err();
        assert_eq!(error.to_string(), "simulated write failure");
        assert!(!path.exists());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);

        save_pdf(&path, b"%PDF-complete").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"%PDF-complete");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn destination_created_during_write_is_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("invoice.pdf");
        let error = write_new_pdf(&path, |file| {
            file.write_all(b"%PDF-new")?;
            fs::write(&path, b"existing file")?;
            Ok(())
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&path).unwrap(), b"existing file");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn persisted_pdf_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("invoice.pdf");
        save_pdf(&path, b"%PDF-private").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
