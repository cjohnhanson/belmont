use crate::error::{Error, Result};

use super::Backend;

/// The backend that reads and writes secrets in the OS credential store. The
/// store is the macOS Keychain, the Windows Credential Manager, or the Linux
/// secret-service.
pub struct KeyringBackend;

/// Parse a `SERVICE/ACCOUNT` path. Return the service and the account.
fn parse_path(path: &str) -> Result<(&str, &str)> {
    let (service, account) = path
        .split_once('/')
        .ok_or_else(|| Error::InvalidRefUri(format!("ref+keyring://{path} — the path must be SERVICE/ACCOUNT")))?;

    if service.is_empty() || account.is_empty() {
        return Err(Error::InvalidRefUri(format!(
            "ref+keyring://{path} — the service and the account must not be empty"
        )));
    }

    Ok((service, account))
}

impl Backend for KeyringBackend {
    fn resolve(&self, path: &str) -> Result<String> {
        let (service, account) = parse_path(path)?;

        let entry = keyring::Entry::new(service, account)
            .map_err(|e| Error::KeyringError(format!("{service}/{account}: {e}")))?;

        entry
            .get_password()
            .map_err(|e| Error::KeyringError(format!("{service}/{account}: {e}")))
    }

    fn set(&self, path: &str, value: &str) -> Result<()> {
        let (service, account) = parse_path(path)?;

        let entry = keyring::Entry::new(service, account)
            .map_err(|e| Error::KeyringError(format!("{service}/{account}: {e}")))?;

        entry
            .set_password(value)
            .map_err(|e| Error::KeyringError(format!("{service}/{account}: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_slash_fails() {
        let backend = KeyringBackend;
        let err = backend.resolve("no-slash").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn empty_service_fails() {
        let backend = KeyringBackend;
        let err = backend.resolve("/account").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn empty_account_fails() {
        let backend = KeyringBackend;
        let err = backend.resolve("service/").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn nonexistent_entry_returns_error() {
        let backend = KeyringBackend;
        let result = backend.resolve("belmont-test-nonexistent/does-not-exist");
        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn roundtrip_set_then_resolve() {
        let backend = KeyringBackend;
        let path = "belmont-test/roundtrip-test";

        // Skip the test if no keychain is available, for example in the nix sandbox.
        if let Err(e) = backend.set(path, "test-value-12345") {
            eprintln!("skipping roundtrip test: {e}");
            return;
        }

        // Resolve the value
        let value = backend.resolve(path).unwrap();
        assert_eq!(value, "test-value-12345");

        // Clean up
        let (service, account) = parse_path(path).unwrap();
        let entry = keyring::Entry::new(service, account).unwrap();
        let _ = entry.delete_credential();
    }
}
