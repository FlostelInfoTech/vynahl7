//! TLS/mTLS configuration for secure communication
//!
//! This module provides utilities for configuring TLS and mutual TLS (mTLS)
//! for both HTTP clients and servers.

#[cfg(feature = "tls")]
use crate::{Error, Result};

#[cfg(feature = "tls")]
use std::path::Path;
#[cfg(feature = "tls")]
use std::sync::Arc;

#[cfg(feature = "tls")]
use rustls::crypto::CryptoProvider;
#[cfg(feature = "tls")]
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
#[cfg(feature = "tls")]
use rustls::{ClientConfig, ConfigBuilder, RootCertStore, ServerConfig, WantsVerifier};
/// Context wrapper preserving the file path (when any) alongside the
/// underlying TLS failure so errors report *what* failed, not just how.
#[cfg(feature = "tls")]
#[derive(Debug)]
struct TlsLoadError {
    context: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

#[cfg(feature = "tls")]
impl std::fmt::Display for TlsLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.source {
            Some(source) => write!(f, "{}: {source}", self.context),
            None => f.write_str(&self.context),
        }
    }
}

#[cfg(feature = "tls")]
impl std::error::Error for TlsLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|s| s as &(dyn std::error::Error + 'static))
    }
}

/// Wrap a TLS load/parse/config failure as [`Error::Io`], preserving the
/// path-bearing context and the underlying cause.
#[cfg(feature = "tls")]
fn tls_io_error(
    kind: std::io::ErrorKind,
    context: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
) -> Error {
    Error::Io(std::io::Error::new(kind, TlsLoadError { context, source }))
}

/// Wrap a rustls configuration failure, keeping the underlying cause.
#[cfg(feature = "tls")]
fn config_error(context: &str, e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Error {
    tls_io_error(
        std::io::ErrorKind::InvalidData,
        context.to_string(),
        Some(e.into()),
    )
}

/// Wrap a file read failure, keeping the path, the I/O kind, and the cause.
#[cfg(feature = "tls")]
fn read_error(path: &Path, what: &str, e: std::io::Error) -> Error {
    let kind = e.kind();
    tls_io_error(
        kind,
        format!("Failed to read {what} file '{}'", path.display()),
        Some(Box::new(e)),
    )
}

/// Load PEM certificates via `rustls::pki_types` (replaces the
/// unmaintained rustls-pemfile crate; same file-in, DER-out contract).
#[cfg(feature = "tls")]
fn load_certs(path: impl AsRef<Path>, what: &str) -> Result<Vec<CertificateDer<'static>>> {
    let path: &Path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|e| read_error(path, what, e))?;
    CertificateDer::pem_slice_iter(&bytes)
        .map(|item| {
            item.map(|cert| cert.into_owned()).map_err(|e| {
                tls_io_error(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to parse {what} file '{}'", path.display()),
                    Some(Box::new(e)),
                )
            })
        })
        .collect()
}

/// Load the first PEM private key (PKCS#8, PKCS#1, or SEC1).
#[cfg(feature = "tls")]
fn load_private_key(path: impl AsRef<Path>, what: &str) -> Result<PrivateKeyDer<'static>> {
    let path: &Path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|e| read_error(path, what, e))?;
    PrivateKeyDer::pem_slice_iter(&bytes)
        .next()
        .transpose()
        .map_err(|e| {
            tls_io_error(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse {what} file '{}'", path.display()),
                Some(Box::new(e)),
            )
        })?
        .map(|k| k.clone_key())
        .ok_or_else(|| {
            tls_io_error(
                std::io::ErrorKind::InvalidData,
                format!("No private key found in {what} file '{}'", path.display()),
                None,
            )
        })
}

/// Parse PEM certificates from in-memory bytes (same DER-out contract as
/// [`load_certs`], without the file read — for material sourced from a
/// database row, KMS, or secret manager rather than disk).
#[cfg(feature = "tls")]
fn parse_certs_pem(pem: &[u8], what: &str) -> Result<Vec<CertificateDer<'static>>> {
    CertificateDer::pem_slice_iter(pem)
        .map(|item| {
            item.map(|cert| cert.into_owned()).map_err(|e| {
                tls_io_error(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to parse {what} PEM"),
                    Some(Box::new(e)),
                )
            })
        })
        .collect()
}

/// Parse the first PEM private key (PKCS#8, PKCS#1, or SEC1) from
/// in-memory bytes. Byte-slice twin of [`load_private_key`].
#[cfg(feature = "tls")]
fn parse_private_key_pem(pem: &[u8], what: &str) -> Result<PrivateKeyDer<'static>> {
    PrivateKeyDer::pem_slice_iter(pem)
        .next()
        .transpose()
        .map_err(|e| {
            tls_io_error(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse {what} PEM"),
                Some(Box::new(e)),
            )
        })?
        .map(|k| k.clone_key())
        .ok_or_else(|| {
            tls_io_error(
                std::io::ErrorKind::InvalidData,
                format!("No private key found in {what} PEM"),
                None,
            )
        })
}

/// The rustls [`CryptoProvider`] this crate builds its TLS configurations with.
///
/// rustls' `ClientConfig::builder()`, `ServerConfig::builder()` and
/// `WebPkiClientVerifier::builder()` all resolve a *process-level* provider, and
/// **panic** when the crate features name more than one:
/// `CryptoProvider::from_crate_features()` returns `None` for an ambiguous feature
/// set, and its caller `expect`s.
///
/// That is not hypothetical. This crate compiles `aws-lc-rs` in (a rustls
/// default feature), and any downstream build that additionally enables `ring` —
/// an application linking reqwest alongside ldap3, say — makes every TLS-enabled
/// configuration panic the moment it is constructed.
///
/// So choose explicitly rather than infer. An application that deliberately
/// installed a provider — a FIPS build, say — still wins; otherwise take
/// `aws-lc-rs`, which this crate's own rustls dependency guarantees is compiled
/// in (it is a rustls default feature) and which rustls itself prefers.
#[cfg(feature = "tls")]
pub(crate) fn crypto_provider() -> Arc<CryptoProvider> {
    CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| Arc::new(rustls::crypto::aws_lc_rs::default_provider()))
}

/// Begin a client configuration on [`crypto_provider`].
///
/// Equivalent to `ClientConfig::builder()`, which is itself
/// `builder_with_provider(..).with_protocol_versions(DEFAULT_VERSIONS).unwrap()`,
/// except that the provider is chosen instead of inferred. The `expect` is
/// unreachable for the same reason rustls' own is: it fires only if the provider
/// supports none of the default protocol versions.
#[cfg(feature = "tls")]
pub(crate) fn client_config_builder() -> ConfigBuilder<ClientConfig, WantsVerifier> {
    ClientConfig::builder_with_provider(crypto_provider())
        .with_safe_default_protocol_versions()
        .expect("the rustls provider supports none of the default TLS protocol versions")
}

/// Begin a server configuration on [`crypto_provider`]. See [`client_config_builder`].
#[cfg(feature = "tls")]
pub(crate) fn server_config_builder() -> ConfigBuilder<ServerConfig, WantsVerifier> {
    ServerConfig::builder_with_provider(crypto_provider())
        .with_safe_default_protocol_versions()
        .expect("the rustls provider supports none of the default TLS protocol versions")
}

/// TLS configuration for client connections
#[cfg(feature = "tls")]
#[derive(Clone)]
pub struct TlsClientConfig {
    pub(crate) config: Arc<ClientConfig>,
}

#[cfg(feature = "tls")]
impl TlsClientConfig {
    /// Create a new TLS client configuration with system root certificates
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::tls::TlsClientConfig;
    ///
    /// let config = TlsClientConfig::new().unwrap();
    /// # }
    /// ```
    pub fn new() -> Result<Self> {
        let mut root_store = RootCertStore::empty();

        // Add system root certificates
        for cert in webpki_roots::TLS_SERVER_ROOTS.iter() {
            root_store.roots.push(cert.clone());
        }

        let config = client_config_builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Create a TLS client configuration with custom root CA certificate
    ///
    /// # Arguments
    ///
    /// * `ca_cert_path` - Path to the PEM-encoded CA certificate file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::tls::TlsClientConfig;
    ///
    /// let config = TlsClientConfig::with_ca_cert("ca-cert.pem").unwrap();
    /// # }
    /// ```
    pub fn with_ca_cert(ca_cert_path: impl AsRef<Path>) -> Result<Self> {
        let ca_cert_path = ca_cert_path.as_ref();
        let mut root_store = RootCertStore::empty();

        // Load CA certificate
        let ca_certs = load_certs(ca_cert_path, "CA certificate")?;
        if ca_certs.is_empty() {
            return Err(tls_io_error(
                std::io::ErrorKind::InvalidData,
                format!(
                    "No certificates found in CA certificate file '{}'",
                    ca_cert_path.display()
                ),
                None,
            ));
        }

        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| config_error("Failed to add CA certificate", e))?;
        }

        let config = client_config_builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Create a TLS client configuration with a custom root CA
    /// sourced as PEM bytes. Equivalent to [`Self::with_ca_cert`]
    /// but accepts in-memory bytes (DB, KMS, secret manager).
    ///
    /// # Arguments
    ///
    /// * `ca_pem` - PEM-encoded CA certificate bytes
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::tls::TlsClientConfig;
    ///
    /// let ca_pem = std::fs::read("ca-cert.pem").unwrap();
    /// let config = TlsClientConfig::with_ca_cert_pem(&ca_pem).unwrap();
    /// # }
    /// ```
    pub fn with_ca_cert_pem(ca_pem: &[u8]) -> Result<Self> {
        let mut root_store = RootCertStore::empty();

        let ca_certs = parse_certs_pem(ca_pem, "CA certificate")?;
        if ca_certs.is_empty() {
            return Err(tls_io_error(
                std::io::ErrorKind::InvalidData,
                "No certificates found in CA certificate PEM".to_string(),
                None,
            ));
        }

        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| config_error("Failed to add CA certificate", e))?;
        }

        let config = client_config_builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Create a TLS client configuration with mutual TLS (client certificate)
    ///
    /// # Arguments
    ///
    /// * `ca_cert_path` - Path to the PEM-encoded CA certificate file
    /// * `client_cert_path` - Path to the PEM-encoded client certificate file
    /// * `client_key_path` - Path to the PEM-encoded client private key file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::tls::TlsClientConfig;
    ///
    /// let config = TlsClientConfig::with_mtls(
    ///     "ca-cert.pem",
    ///     "client-cert.pem",
    ///     "client-key.pem"
    /// ).unwrap();
    /// # }
    /// ```
    pub fn with_mtls(
        ca_cert_path: impl AsRef<Path>,
        client_cert_path: impl AsRef<Path>,
        client_key_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let ca_cert_path = ca_cert_path.as_ref();
        let mut root_store = RootCertStore::empty();

        // Load CA certificate
        let ca_certs = load_certs(ca_cert_path, "CA certificate")?;
        if ca_certs.is_empty() {
            return Err(tls_io_error(
                std::io::ErrorKind::InvalidData,
                format!(
                    "No certificates found in CA certificate file '{}'",
                    ca_cert_path.display()
                ),
                None,
            ));
        }

        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| config_error("Failed to add CA certificate", e))?;
        }

        // Load client certificate
        let client_certs: Vec<CertificateDer> = load_certs(client_cert_path, "client certificate")?;

        // Load client private key
        let client_key = load_private_key(client_key_path, "private key")?;

        let config = client_config_builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(client_certs, client_key)
            .map_err(|e| config_error("Failed to configure client auth", e))?;

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Create a mutual-TLS client configuration from PEM bytes.
    /// Equivalent to [`Self::with_mtls`] but accepts in-memory bytes
    /// (DB, KMS, secret manager).
    ///
    /// # Arguments
    ///
    /// * `ca_pem` - PEM-encoded CA certificate bytes
    /// * `client_cert_pem` - PEM-encoded client certificate bytes
    /// * `client_key_pem` - PEM-encoded client private key bytes
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::tls::TlsClientConfig;
    ///
    /// let ca_pem = std::fs::read("ca-cert.pem").unwrap();
    /// let cert_pem = std::fs::read("client-cert.pem").unwrap();
    /// let key_pem = std::fs::read("client-key.pem").unwrap();
    /// let config =
    ///     TlsClientConfig::with_mtls_pem(&ca_pem, &cert_pem, &key_pem).unwrap();
    /// # }
    /// ```
    pub fn with_mtls_pem(
        ca_pem: &[u8],
        client_cert_pem: &[u8],
        client_key_pem: &[u8],
    ) -> Result<Self> {
        let mut root_store = RootCertStore::empty();

        let ca_certs = parse_certs_pem(ca_pem, "CA certificate")?;
        if ca_certs.is_empty() {
            return Err(tls_io_error(
                std::io::ErrorKind::InvalidData,
                "No certificates found in CA certificate PEM".to_string(),
                None,
            ));
        }

        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| config_error("Failed to add CA certificate", e))?;
        }

        let client_certs: Vec<CertificateDer> =
            parse_certs_pem(client_cert_pem, "client certificate")?;

        let client_key = parse_private_key_pem(client_key_pem, "private key")?;

        let config = client_config_builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(client_certs, client_key)
            .map_err(|e| config_error("Failed to configure client auth", e))?;

        Ok(Self {
            config: Arc::new(config),
        })
    }
}

/// TLS configuration for server connections
#[cfg(feature = "tls")]
#[derive(Clone)]
pub struct TlsServerConfig {
    pub(crate) config: Arc<ServerConfig>,
}

#[cfg(feature = "tls")]
impl TlsServerConfig {
    /// Create a new TLS server configuration
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the PEM-encoded server certificate file
    /// * `key_path` - Path to the PEM-encoded server private key file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::tls::TlsServerConfig;
    ///
    /// let config = TlsServerConfig::new("server-cert.pem", "server-key.pem").unwrap();
    /// # }
    /// ```
    pub fn new(cert_path: impl AsRef<Path>, key_path: impl AsRef<Path>) -> Result<Self> {
        // Load server certificate
        let certs: Vec<CertificateDer> = load_certs(cert_path, "server certificate")?;

        // Load server private key
        let key = load_private_key(key_path, "private key")?;

        let config = server_config_builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| config_error("Failed to configure server TLS", e))?;

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Create a TLS server configuration from PEM bytes.
    ///
    /// Equivalent to [`Self::new`] but takes the cert and key as
    /// in-memory PEM buffers — useful when the material is sourced
    /// from a database row or KMS rather than a file on disk.
    ///
    /// # Arguments
    ///
    /// * `cert_pem` - PEM-encoded server certificate bytes
    /// * `key_pem` - PEM-encoded server private key bytes
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::tls::TlsServerConfig;
    ///
    /// let cert_pem = std::fs::read("server-cert.pem").unwrap();
    /// let key_pem = std::fs::read("server-key.pem").unwrap();
    /// let config = TlsServerConfig::new_pem(&cert_pem, &key_pem).unwrap();
    /// # }
    /// ```
    pub fn new_pem(cert_pem: &[u8], key_pem: &[u8]) -> Result<Self> {
        let certs: Vec<CertificateDer> = parse_certs_pem(cert_pem, "server certificate")?;

        let key = parse_private_key_pem(key_pem, "private key")?;

        let config = server_config_builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| config_error("Failed to configure server TLS", e))?;

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Create a TLS server configuration with mutual TLS (client certificate verification)
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the PEM-encoded server certificate file
    /// * `key_path` - Path to the PEM-encoded server private key file
    /// * `ca_cert_path` - Path to the PEM-encoded CA certificate file for client verification
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::tls::TlsServerConfig;
    ///
    /// let config = TlsServerConfig::with_mtls(
    ///     "server-cert.pem",
    ///     "server-key.pem",
    ///     "ca-cert.pem"
    /// ).unwrap();
    /// # }
    /// ```
    pub fn with_mtls(
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
        ca_cert_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let ca_cert_path = ca_cert_path.as_ref();
        // Load CA certificate for client verification
        let mut client_auth_roots = RootCertStore::empty();
        let ca_certs = load_certs(ca_cert_path, "CA certificate")?;
        if ca_certs.is_empty() {
            return Err(tls_io_error(
                std::io::ErrorKind::InvalidData,
                format!(
                    "No certificates found in CA certificate file '{}'",
                    ca_cert_path.display()
                ),
                None,
            ));
        }

        for cert in ca_certs {
            client_auth_roots
                .add(cert)
                .map_err(|e| config_error("Failed to add CA certificate", e))?;
        }

        // Load server certificate
        let certs: Vec<CertificateDer> = load_certs(cert_path, "server certificate")?;

        // Load server private key
        let key = load_private_key(key_path, "private key")?;

        let client_cert_verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(client_auth_roots),
            crypto_provider(),
        )
        .build()
        .map_err(|e| config_error("Failed to build client verifier", e))?;

        let config = server_config_builder()
            .with_client_cert_verifier(client_cert_verifier)
            .with_single_cert(certs, key)
            .map_err(|e| config_error("Failed to configure server TLS", e))?;

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Create a mutual-TLS server configuration from PEM bytes.
    ///
    /// Same as [`Self::with_mtls`] but sources cert / key / client
    /// CA as in-memory PEM buffers.
    ///
    /// # Arguments
    ///
    /// * `cert_pem` - PEM-encoded server certificate bytes
    /// * `key_pem` - PEM-encoded server private key bytes
    /// * `ca_pem` - PEM-encoded CA certificate bytes for client verification
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::tls::TlsServerConfig;
    ///
    /// let cert_pem = std::fs::read("server-cert.pem").unwrap();
    /// let key_pem = std::fs::read("server-key.pem").unwrap();
    /// let ca_pem = std::fs::read("ca-cert.pem").unwrap();
    /// let config =
    ///     TlsServerConfig::with_mtls_pem(&cert_pem, &key_pem, &ca_pem).unwrap();
    /// # }
    /// ```
    pub fn with_mtls_pem(cert_pem: &[u8], key_pem: &[u8], ca_pem: &[u8]) -> Result<Self> {
        let mut client_auth_roots = RootCertStore::empty();
        let ca_certs = parse_certs_pem(ca_pem, "CA certificate")?;
        if ca_certs.is_empty() {
            return Err(tls_io_error(
                std::io::ErrorKind::InvalidData,
                "No certificates found in CA certificate PEM".to_string(),
                None,
            ));
        }

        for cert in ca_certs {
            client_auth_roots
                .add(cert)
                .map_err(|e| config_error("Failed to add CA certificate", e))?;
        }

        let certs: Vec<CertificateDer> = parse_certs_pem(cert_pem, "server certificate")?;

        let key = parse_private_key_pem(key_pem, "private key")?;

        let client_cert_verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(client_auth_roots),
            crypto_provider(),
        )
        .build()
        .map_err(|e| config_error("Failed to build client verifier", e))?;

        let config = server_config_builder()
            .with_client_cert_verifier(client_cert_verifier)
            .with_single_cert(certs, key)
            .map_err(|e| config_error("Failed to configure server TLS", e))?;

        Ok(Self {
            config: Arc::new(config),
        })
    }
}

#[cfg(feature = "tls")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_new() {
        let config = TlsClientConfig::new();
        assert!(config.is_ok());
    }

    const TEST_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIDETCCAfmgAwIBAgIUOE+w0g1S/G2rwRB/8suIUFaZprkwDQYJKoZIhvcNAQEL
BQAwGDEWMBQGA1UEAwwNcnM3LWh0dHAtdGVzdDAeFw0yNjEwMDMxMjQ1MzhaFw0y
NjEwMDUxMjQ1MzhaMBgxFjAUBgNVBAMMDXJzNy1odHRwLXRlc3QwggEiMA0GCSqG
SIb3DQEBAQUAA4IBDwAwggEKAoIBAQCUIJCmuiKGYyOKmf08cX0H+HXONmwxZyiU
dbCmLg6p5BiJhVXZ3rVj2Eb7or2O6NCzqZb3p/MNevjovTyUnfgWF8uSL9d+3XL6
tgeLUf2ousdz1aznSevG30ozS/q2+NmqXNW1DVaQod3PeZqsapr2JE5hqSrNouIi
6uS1pG0qb6e5Ex/Ui8TqzyxOHUC07S8UThtEYG8cegA+qMrP78My3yJltrBCwIsf
auW1z7rDDiaPIFMtgz88YoI6suLCAxA+eQiUc+qj5CH+yem9aQuLkuG6uS5BVfws
LUe+sfXRp4sfkVQZM/qzEF2wn5uYUOJzDKy6bFVgQnnN3Pqm40R9AgMBAAGjUzBR
MB0GA1UdDgQWBBTMO0D6LVxVlHJpZ0FhceooWw25MDAfBgNVHSMEGDAWgBTMO0D6
LVxVlHJpZ0FhceooWw25MDAPBgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3DQEBCwUA
A4IBAQAzZtmaUN7xHIj2E5JJ4I5u/ycUqY8xWubMjOaQGAmwhhC8StoULnNYHcI0
bqoBJeUXtJFA3XMmRaMSdw/6Mpw1XqlPvDHrk5voRzOu/VRrNb2u7bYufXxwN8vq
RxShxIXnhbG8mWe8XjtywqvBcIWPZj/yOyaMAYOd21km9VbAZudblcnVUD5lGP2H
Gz5n3+Q99ZalZZMW/db1PnYOtLo6Gxm3udxRATVhDYu3P/aLGHZPqTWCW/gbuSSl
fMaEELAbZxTgjtSWMK0GC7VhF7KywfcV1wtRkb9bkoFCgmqG7RgB9s1kRHBDUAXI
6By6bE8HWWS8kH6xI3xdNaJ+bDSP
-----END CERTIFICATE-----
"#;
    const TEST_KEY_PKCS8_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCUIJCmuiKGYyOK
mf08cX0H+HXONmwxZyiUdbCmLg6p5BiJhVXZ3rVj2Eb7or2O6NCzqZb3p/MNevjo
vTyUnfgWF8uSL9d+3XL6tgeLUf2ousdz1aznSevG30ozS/q2+NmqXNW1DVaQod3P
eZqsapr2JE5hqSrNouIi6uS1pG0qb6e5Ex/Ui8TqzyxOHUC07S8UThtEYG8cegA+
qMrP78My3yJltrBCwIsfauW1z7rDDiaPIFMtgz88YoI6suLCAxA+eQiUc+qj5CH+
yem9aQuLkuG6uS5BVfwsLUe+sfXRp4sfkVQZM/qzEF2wn5uYUOJzDKy6bFVgQnnN
3Pqm40R9AgMBAAECggEACAIGr+zKJwe/CSQwC497Ge41uDM7ACS6jeQa67D7u/WH
2g9qR8ABgGdV1w7CPrKZbc0SS0s6GZV3VWW9m686lT8uEzYg6UvhhmpG2CJTm0aV
t85qbYqSg4rYwRMvd6rWYZ5pM0OIVYNasXe6g+2zePEXh7IUjU9r+EVIngVInY2c
qgks/lDUTh0W76kIh0AWUnbT0Mxlln8K91l4R4x5V3s19Yi/1DNlPaXSdWFRk+zY
HG8Q0gvH0Wb/tGx0X5djuGypXsmN9YC4Sim7bWJMJsKsTwFcPgPrmtcPA+7Sgo7p
BBfPi8vCWhtmdwBIsZ6N49jc9LtGJDSFMkDiaGsW5QKBgQDK2BAdPpLHE2+lq9ad
x2DCT/aq3ApXKtapCO5j0UEBbjYeteiUuygVbasz3dzNVLIbimw7nG1T5j3THRsu
VIZv27OKs4G3yRRCoNi5GkIVR9wjJYAAvpiAK/5aJT58F7Qw3OGuuJ/usAXJnknZ
HYJWVXu+mGEzSGjv1x0jxfuEMwKBgQC68clZLyqWA8gTVS6P1QJyy+rkQnWRoRmp
Q2zTvM3DJO+4alEUc8gLQmYps0JpQkDfWy4c1VLEmDKfHd6/C74jr9cQhjL1JQqP
BxcCUDkhr4f9ilY02oS4ImJiXUY2Nmp1t5uDTR6fIg6C2IyNFxOooJUinphBtM/4
5FGHvL3kjwKBgQDGtXyruWW+QYsI59qZu/xMvlrtLBuxIpUhCE/IVvBwf0/jK97I
/MMaotGpQLmSKFtGYYb/qpDrNMzx6Dx9WW5NXBN5V7pGmWQQlVNZmaCNClVb1Nud
XxkzzmUgWAXrOKlbiLZJHDIIeEVjKn47hA3vl1Re9RWbUF8EPV4dclCfQwKBgDk9
Aqs918oqyj2xHRqI28EOUpMlDh5StyquLEoASvfu/T73vm+Gq79WyzUFX9YepQI+
RrYpUrhC6WzyllrkW4lNs4mprTyF5N+6Cf95VKTNu9OTYaW5Kl2+mWUIrhC3RI1a
dFEVccTDhrHlX9NrN2ewEzmQWYn0tKC+N4BZ98JdAoGAKIea4MnUkAHHIEqiPrr3
e3KGnd86XMVsVuJ6CWoBmKJAYpvJCyrcPhG/DdI3GTecWvXWKKfUi7YQSPZdLeYU
dvm/Tf1+TMiiULgzTvXV6m2bApxVDRN/FgKcLFv+AkX96UoZThn2c2WyGGUlNdDZ
k52+4SYn6TqRmMB18fQxeG0=
-----END PRIVATE KEY-----
"#;
    const TEST_KEY_PKCS1_PEM: &str = r#"-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEAlCCQproihmMjipn9PHF9B/h1zjZsMWcolHWwpi4OqeQYiYVV
2d61Y9hG+6K9jujQs6mW96fzDXr46L08lJ34FhfLki/Xft1y+rYHi1H9qLrHc9Ws
50nrxt9KM0v6tvjZqlzVtQ1WkKHdz3marGqa9iROYakqzaLiIurktaRtKm+nuRMf
1IvE6s8sTh1AtO0vFE4bRGBvHHoAPqjKz+/DMt8iZbawQsCLH2rltc+6ww4mjyBT
LYM/PGKCOrLiwgMQPnkIlHPqo+Qh/snpvWkLi5LhurkuQVX8LC1HvrH10aeLH5FU
GTP6sxBdsJ+bmFDicwysumxVYEJ5zdz6puNEfQIDAQABAoIBAAgCBq/syicHvwkk
MAuPexnuNbgzOwAkuo3kGuuw+7v1h9oPakfAAYBnVdcOwj6ymW3NEktLOhmVd1Vl
vZuvOpU/LhM2IOlL4YZqRtgiU5tGlbfOam2KkoOK2METL3eq1mGeaTNDiFWDWrF3
uoPts3jxF4eyFI1Pa/hFSJ4FSJ2NnKoJLP5Q1E4dFu+pCIdAFlJ209DMZZZ/CvdZ
eEeMeVd7NfWIv9QzZT2l0nVhUZPs2BxvENILx9Fm/7RsdF+XY7hsqV7JjfWAuEop
u21iTCbCrE8BXD4D65rXDwPu0oKO6QQXz4vLwlobZncASLGejePY3PS7RiQ0hTJA
4mhrFuUCgYEAytgQHT6SxxNvpavWncdgwk/2qtwKVyrWqQjuY9FBAW42HrXolLso
FW2rM93czVSyG4psO5xtU+Y90x0bLlSGb9uzirOBt8kUQqDYuRpCFUfcIyWAAL6Y
gCv+WiU+fBe0MNzhrrif7rAFyZ5J2R2CVlV7vphhM0ho79cdI8X7hDMCgYEAuvHJ
WS8qlgPIE1Uuj9UCcsvq5EJ1kaEZqUNs07zNwyTvuGpRFHPIC0JmKbNCaUJA31su
HNVSxJgynx3evwu+I6/XEIYy9SUKjwcXAlA5Ia+H/YpWNNqEuCJiYl1GNjZqdbeb
g00enyIOgtiMjRcTqKCVIp6YQbTP+ORRh7y95I8CgYEAxrV8q7llvkGLCOfambv8
TL5a7SwbsSKVIQhPyFbwcH9P4yveyPzDGqLRqUC5kihbRmGG/6qQ6zTM8eg8fVlu
TVwTeVe6RplkEJVTWZmgjQpVW9TbnV8ZM85lIFgF6zipW4i2SRwyCHhFYyp+O4QN
75dUXvUVm1BfBD1eHXJQn0MCgYA5PQKrPdfKKso9sR0aiNvBDlKTJQ4eUrcqrixK
AEr37v0+975vhqu/Vss1BV/WHqUCPka2KVK4Quls8pZa5FuJTbOJqa08heTfugn/
eVSkzbvTk2GluSpdvpllCK4Qt0SNWnRRFXHEw4ax5V/TazdnsBM5kFmJ9LSgvjeA
WffCXQKBgCiHmuDJ1JABxyBKoj6693tyhp3fOlzFbFbieglqAZiiQGKbyQsq3D4R
vw3SNxk3nFr11iin1Iu2EEj2XS3mFHb5v039fkzIolC4M0711eptmwKcVQ0TfxYC
nCxb/gJF/elKGU4Z9nNlshhlJTXQ2ZOdvuEmJ+k6kZjAdfH0MXht
-----END RSA PRIVATE KEY-----
"#;

    fn write_fixture(name: &str, contents: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vyna_hl7_http_tls_test_{}_{}",
            std::process::id(),
            name
        ));
        std::fs::write(&path, contents).expect("write fixture");
        path
    }

    #[test]
    fn test_load_certs_single() {
        let path = write_fixture("cert.pem", TEST_CERT_PEM);
        let certs = load_certs(&path, "certificate").expect("parse cert");
        assert_eq!(certs.len(), 1);
        assert!(!certs[0].is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_private_key_pkcs8_and_pkcs1() {
        for (name, pem, expect_pkcs8) in [
            ("key8.pem", TEST_KEY_PKCS8_PEM, true),
            ("key1.pem", TEST_KEY_PKCS1_PEM, false),
        ] {
            let path = write_fixture(name, pem);
            let key = load_private_key(&path, "private key").expect("parse key");
            assert_eq!(
                matches!(key, PrivateKeyDer::Pkcs8(_)),
                expect_pkcs8,
                "{name}"
            );
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn test_missing_files_report_io_error_with_path() {
        let missing =
            std::env::temp_dir().join(format!("vyna_hl7_tls_missing_{}.pem", std::process::id()));
        assert!(!missing.exists());
        let err = match TlsServerConfig::new(&missing, &missing) {
            Ok(_) => panic!("expected an error for missing files"),
            Err(e) => e,
        };
        match err {
            Error::Io(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
                assert!(e.to_string().contains(missing.to_str().unwrap()));
            }
            other => panic!("expected Error::Io, got: {other}"),
        }
    }

    #[test]
    fn test_garbage_key_reports_parse_error_with_path() {
        let path = write_fixture("junk-key.pem", "not a pem file\n");
        let err = load_private_key(&path, "private key").unwrap_err();
        match err {
            Error::Io(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::InvalidData);
                assert!(e.to_string().contains(path.to_str().unwrap()));
            }
            other => panic!("expected Error::Io, got: {other}"),
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_pem_loaders_reject_garbage() {
        let path = write_fixture("bad.pem", "not a pem file\n");
        // No PEM blocks: empty vec, same as the old rustls-pemfile iterator.
        let certs = load_certs(&path, "certificate").expect("junk yields no certs");
        assert!(certs.is_empty());
        assert!(load_private_key(&path, "private key").is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_with_ca_cert_rejects_file_without_certificates() {
        for (name, contents) in [("junk.pem", "not a pem file\n"), ("empty.pem", "")] {
            let path = write_fixture(name, contents);
            let err = match TlsClientConfig::with_ca_cert(&path) {
                Ok(_) => panic!("{name}: expected an error for a CA file without certificates"),
                Err(e) => e,
            };
            let message = err.to_string();
            assert!(
                message.contains("No certificates found"),
                "{name}: {message}"
            );
            assert!(message.contains(name), "{name}: {message}");
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn test_client_with_mtls_rejects_empty_ca_file() {
        let ca = write_fixture("junk-ca-client.pem", "not a pem file\n");
        let missing = std::env::temp_dir().join(format!(
            "vyna_hl7_tls_missing_client_{}.pem",
            std::process::id()
        ));
        assert!(!missing.exists());
        let err = match TlsClientConfig::with_mtls(&ca, &missing, &missing) {
            Ok(_) => panic!("expected an error for an mTLS CA file without certificates"),
            Err(e) => e,
        };
        let message = err.to_string();
        assert!(message.contains("No certificates found"), "{message}");
        assert!(message.contains("junk-ca-client.pem"), "{message}");
        let _ = std::fs::remove_file(&ca);
    }

    #[test]
    fn test_server_with_mtls_rejects_empty_ca_file() {
        let ca = write_fixture("junk-ca-server.pem", "not a pem file\n");
        let missing = std::env::temp_dir().join(format!(
            "vyna_hl7_tls_missing_server_{}.pem",
            std::process::id()
        ));
        assert!(!missing.exists());
        let err = match TlsServerConfig::with_mtls(&missing, &missing, &ca) {
            Ok(_) => panic!("expected an error for an mTLS CA file without certificates"),
            Err(e) => e,
        };
        let message = err.to_string();
        assert!(message.contains("No certificates found"), "{message}");
        assert!(message.contains("junk-ca-server.pem"), "{message}");
        let _ = std::fs::remove_file(&ca);
    }

    // Second, unrelated key for the key-mismatch negative below
    // (TEST_KEY_PKCS8_PEM and TEST_KEY_PKCS1_PEM are the same key in
    // two encodings, both matching TEST_CERT_PEM). Test-only.
    const TEST_OTHER_KEY_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQDFqfEG+m9yBZph
DruDhqrOa8syO0WutFZZBKbaNFT8qnFYwJlcYsJ8SPKs4IhgXtV2XGm0TPd0lTIh
VueP2FkBwfqKNpMuWDbB0eobXlSeYlzWJ88C5ZJBtCZCMOnsNbwBVQEknrlriNPJ
clyFRDgRIKWJCAu3GN1OHRdbRxWDzlVW36iR1s+bmxB6B1X3tpHWFYk63AMAVfNA
FgYdmFdhFQBu8ZLYkxK8PJebo5qc4a/zI6arw402ZbAkUZw8UNLqFpUr1hBaduGm
qzK2sy3VQTYV9x5Lg0iQFL/itiR+yajIDqm9G8A2OBD0XGrkkgALrym7yN4SFEop
X1uxVuupAgMBAAECggEAATt38mtMW0GEBRmwUxhEyUz1MLA+mMh1aNfmS+2CPdtU
P/wslNOaPu93kZIM+19B8cOABbb7/MfGobZ6+lR0CBzDjE5a7utvYV6a8LVE/c1m
L3SCxCBWaLAA+tJoKp9Jm+XjuI1ryuxBRL1Fk6CRoiFPalYuJEg14dt74uGvmT//
wKLnxl6+PpacH/XW7cSj0Qr8XlGLBLx7yNsezTlKUrKXPeFjOik/XK3F+WFWmIV9
7934HwqLp7hkrWVoLhXzEc5rXzARy4vb7SJU3WJv289MTLEKw9+nCvpyAdS3im2C
eLP0AL7PsH9AuYhc/P9ZXTooAU8+JeE4xLHlqVhM8wKBgQD/JIAVDPtij+j2ZY3W
nxcgES3wlxCb6LH6k8IDszlKbccIHN2jHyvPZv1B564jvhZuLe7ZCAe/xb0r5v/R
aIEoz5gBgejGiW6QclGOOrRaBhBy6arvfed9HH0Qd31doMVCS07GZi7IVDIBhTIr
tz3xP1YGTIwF2kvDBJR2vPJQIwKBgQDGU/368CXUQfjsaFI3Yjrm2K3hCDkPpzOy
/FeQzemdTQGUY75JtCelcT9RnSuEb3gGGg9SPkLWwHcspYWVW/CaXXnaGnTL3DIR
fvVcyrdmhCHPWS8LSH9G6bmzYs6bEIpmg4kQVwkyxVemoCKnSsyBhzRjWmGXu/Ji
U7fLV1orwwKBgBSzHGUY0YhUjpz27kP8Uj76xdRgHSvY/B+NP09Xig1czKn+1Yh3
bfF8tI1VQ46IzH8R4O/gs+g+HbPjINwXc2shNH7i0ETTP5IehDlaz+kB1vHLvV+C
wnYnCop8d0JSWc6gF3sHLUJ5ejkfWkurqS3aaptGOJH8/OqMg5MCj1IrAoGAPAlr
Bg6ydOmvr+j4knly+TS4ljmGBH4KAXGwFE6xHIQWKVR0Yx1dLxD0KQGaDdIslDRJ
VQ0x8I58xRAqOweE80W+wE5n8tpqca9sGTXohOkUzw7hVdoRZWcVZv9PKdO9AcFp
uX+PckYY/YDWXKwg8kj6i+TtTCCCBE/xgGWmDz8CgYAq2wJWwTXJSI6kbBaqkIOk
RZEMn2f81FHLghXRIUZ1xIJIniW1LrejWz6uFn5nckwad3y1ZXAgcNdr73lHOI5r
sLgD1KmYUZlIeCcaW5uk162VcOM8cf9nIRZCrUEHg1t40ruqA+iryjtIqFFHw5EW
hd6+cELw2ZXCriD/uuHnPQ==
-----END PRIVATE KEY-----
"#;

    #[test]
    fn pem_ca_cert_rejects_bytes_without_certificates() {
        for (name, contents) in [("junk", "not a pem file\n"), ("empty", "")] {
            let err = match TlsClientConfig::with_ca_cert_pem(contents.as_bytes()) {
                Ok(_) => panic!("{name}: expected an error for CA bytes without certificates"),
                Err(e) => e,
            };
            match err {
                Error::Io(e) => {
                    assert_eq!(e.kind(), std::io::ErrorKind::InvalidData, "{name}");
                    assert!(
                        e.to_string()
                            .contains("No certificates found in CA certificate PEM"),
                        "{name}: {e}"
                    );
                }
                other => panic!("{name}: expected Error::Io, got: {other}"),
            }
        }
    }

    #[test]
    fn pem_client_mtls_rejects_empty_ca() {
        let err = match TlsClientConfig::with_mtls_pem(
            b"not a pem file\n",
            TEST_CERT_PEM.as_bytes(),
            TEST_KEY_PKCS8_PEM.as_bytes(),
        ) {
            Ok(_) => panic!("expected an error for an mTLS CA without certificates"),
            Err(e) => e,
        };
        assert!(
            err.to_string()
                .contains("No certificates found in CA certificate PEM"),
            "{err}"
        );
    }

    #[test]
    fn pem_server_mtls_rejects_empty_ca() {
        let err = match TlsServerConfig::with_mtls_pem(
            TEST_CERT_PEM.as_bytes(),
            TEST_KEY_PKCS8_PEM.as_bytes(),
            b"not a pem file\n",
        ) {
            Ok(_) => panic!("expected an error for an mTLS CA without certificates"),
            Err(e) => e,
        };
        assert!(
            err.to_string()
                .contains("No certificates found in CA certificate PEM"),
            "{err}"
        );
    }

    #[test]
    fn pem_constructors_accept_valid_material() {
        let cert = TEST_CERT_PEM.as_bytes();
        let key8 = TEST_KEY_PKCS8_PEM.as_bytes();
        let key1 = TEST_KEY_PKCS1_PEM.as_bytes();
        assert!(TlsClientConfig::with_ca_cert_pem(cert).is_ok());
        assert!(TlsClientConfig::with_mtls_pem(cert, cert, key8).is_ok());
        // PKCS#1 keys parse through the same path.
        assert!(TlsClientConfig::with_mtls_pem(cert, cert, key1).is_ok());
        assert!(TlsServerConfig::new_pem(cert, key8).is_ok());
        assert!(TlsServerConfig::with_mtls_pem(cert, key8, cert).is_ok());
    }

    /// Every constructor builds under DUAL providers (`aws-lc-rs` + `ring`).
    ///
    /// The dev-dependencies enable `ring` on top of rustls' default
    /// `aws-lc-rs`, reproducing downstream builds that carry both — the
    /// configuration under which inferring the provider panics.
    /// Referencing both providers directly keeps this honest: if either
    /// feature ever drops off, the test fails to COMPILE rather than
    /// passing over a single-provider build.
    #[test]
    fn tls_constructors_build_under_dual_providers() {
        // Compile guards: both providers must be present for this test to
        // mean anything. (Values unused — only their existence matters.)
        let _ = rustls::crypto::ring::default_provider();
        let _ = rustls::crypto::aws_lc_rs::default_provider();

        let cert = TEST_CERT_PEM.as_bytes();
        let key8 = TEST_KEY_PKCS8_PEM.as_bytes();
        assert!(TlsClientConfig::new().is_ok());
        assert!(TlsClientConfig::with_ca_cert_pem(cert).is_ok());
        assert!(TlsClientConfig::with_mtls_pem(cert, cert, key8).is_ok());
        assert!(TlsServerConfig::new_pem(cert, key8).is_ok());
        assert!(TlsServerConfig::with_mtls_pem(cert, key8, cert).is_ok());

        // The path-taking twins, through temp files.
        let cert_path = write_fixture("dual-cert.pem", TEST_CERT_PEM);
        let key_path = write_fixture("dual-key.pem", TEST_KEY_PKCS8_PEM);
        assert!(TlsClientConfig::with_ca_cert(&cert_path).is_ok());
        assert!(TlsClientConfig::with_mtls(&cert_path, &cert_path, &key_path).is_ok());
        assert!(TlsServerConfig::new(&cert_path, &key_path).is_ok());
        assert!(TlsServerConfig::with_mtls(&cert_path, &key_path, &cert_path).is_ok());
        let _ = std::fs::remove_file(&cert_path);
        let _ = std::fs::remove_file(&key_path);
    }

    #[test]
    fn pem_server_rejects_mismatched_key() {
        let err =
            match TlsServerConfig::new_pem(TEST_CERT_PEM.as_bytes(), TEST_OTHER_KEY_PEM.as_bytes())
            {
                Ok(_) => panic!("expected an error for a key that does not match the cert"),
                Err(e) => e,
            };
        assert!(
            err.to_string().contains("Failed to configure server TLS"),
            "{err}"
        );
    }

    #[test]
    fn pem_client_mtls_rejects_junk_key() {
        let err = match TlsClientConfig::with_mtls_pem(
            TEST_CERT_PEM.as_bytes(),
            TEST_CERT_PEM.as_bytes(),
            b"not a pem file\n",
        ) {
            Ok(_) => panic!("expected an error for a client key without key material"),
            Err(e) => e,
        };
        assert!(
            err.to_string()
                .contains("No private key found in private key PEM"),
            "{err}"
        );
    }
}
