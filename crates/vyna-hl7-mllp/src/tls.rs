//! TLS/mTLS configuration for secure MLLP communication
//!
//! This module provides utilities for configuring TLS and mutual TLS (mTLS)
//! for both MLLP clients and servers over TCP.

#[cfg(feature = "tls")]
use vyna_hl7_core::error::{Error, Result};

#[cfg(feature = "tls")]
use std::path::Path;
#[cfg(feature = "tls")]
use std::sync::Arc;

#[cfg(feature = "tls")]
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
#[cfg(feature = "tls")]
use rustls::{ClientConfig, RootCertStore, ServerConfig};
/// Load PEM certificates via `rustls::pki_types` (replaces the
/// unmaintained rustls-pemfile crate; same file-in, DER-out contract).
#[cfg(feature = "tls")]
fn load_certs(path: impl AsRef<Path>, what: &str) -> Result<Vec<CertificateDer<'static>>> {
    let bytes = std::fs::read(path.as_ref())
        .map_err(|e| Error::Network(format!("Failed to read {what} file: {e}")))?;
    CertificateDer::pem_slice_iter(&bytes)
        .map(|item| {
            item.map(|cert| cert.into_owned())
                .map_err(|e| Error::Network(format!("Failed to parse {what}: {e}")))
        })
        .collect()
}

/// Load the first PEM private key (PKCS#8, PKCS#1, or SEC1).
#[cfg(feature = "tls")]
fn load_private_key(path: impl AsRef<Path>) -> Result<PrivateKeyDer<'static>> {
    let bytes = std::fs::read(path.as_ref())
        .map_err(|e| Error::Network(format!("Failed to read private key file: {e}")))?;
    PrivateKeyDer::pem_slice_iter(&bytes)
        .next()
        .transpose()
        .map_err(|e| Error::Network(format!("Failed to parse private key: {e}")))?
        .map(|k| k.clone_key())
        .ok_or_else(|| Error::Network("No private key found in file".to_string()))
}

/// TLS configuration for MLLP client connections
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
    /// use vyna_hl7_mllp::tls::TlsClientConfig;
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

        let config = ClientConfig::builder()
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
    /// use vyna_hl7_mllp::tls::TlsClientConfig;
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
            return Err(Error::Network(format!(
                "No certificates found in CA certificate file '{}'",
                ca_cert_path.display()
            )));
        }

        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| Error::Network(format!("Failed to add CA certificate: {}", e)))?;
        }

        let config = ClientConfig::builder()
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
    /// use vyna_hl7_mllp::tls::TlsClientConfig;
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
            return Err(Error::Network(format!(
                "No certificates found in CA certificate file '{}'",
                ca_cert_path.display()
            )));
        }

        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| Error::Network(format!("Failed to add CA certificate: {}", e)))?;
        }

        // Load client certificate
        let client_certs: Vec<CertificateDer> = load_certs(client_cert_path, "client certificate")?;

        // Load client private key
        let client_key = load_private_key(client_key_path)?;

        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(client_certs, client_key)
            .map_err(|e| Error::Network(format!("Failed to configure client auth: {}", e)))?;

        Ok(Self {
            config: Arc::new(config),
        })
    }
}

/// TLS configuration for MLLP server connections
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
    /// use vyna_hl7_mllp::tls::TlsServerConfig;
    ///
    /// let config = TlsServerConfig::new("server-cert.pem", "server-key.pem").unwrap();
    /// # }
    /// ```
    pub fn new(
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
    ) -> Result<Self> {
        // Load server certificate
        let certs: Vec<CertificateDer> = load_certs(cert_path, "server certificate")?;

        // Load server private key
        let key = load_private_key(key_path)?;

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| Error::Network(format!("Failed to configure server TLS: {}", e)))?;

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
    /// use vyna_hl7_mllp::tls::TlsServerConfig;
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
            return Err(Error::Network(format!(
                "No certificates found in CA certificate file '{}'",
                ca_cert_path.display()
            )));
        }

        for cert in ca_certs {
            client_auth_roots
                .add(cert)
                .map_err(|e| Error::Network(format!("Failed to add CA certificate: {}", e)))?;
        }

        // Load server certificate
        let certs: Vec<CertificateDer> = load_certs(cert_path, "server certificate")?;

        // Load server private key
        let key = load_private_key(key_path)?;

        let client_cert_verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(client_auth_roots))
            .build()
            .map_err(|e| Error::Network(format!("Failed to build client verifier: {}", e)))?;

        let config = ServerConfig::builder()
            .with_client_cert_verifier(client_cert_verifier)
            .with_single_cert(certs, key)
            .map_err(|e| Error::Network(format!("Failed to configure server TLS: {}", e)))?;

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

    fn write_temp_file(name: &str, contents: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vyna_hl7_mllp_tls_test_{}_{}",
            std::process::id(),
            name
        ));
        std::fs::write(&path, contents).expect("write fixture");
        path
    }

    fn missing_path(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vyna_hl7_mllp_tls_missing_{}_{}",
            std::process::id(),
            name
        ));
        assert!(!path.exists());
        path
    }

    #[test]
    fn test_with_ca_cert_rejects_file_without_certificates() {
        for (name, contents) in [("junk.pem", "not a pem file\n"), ("empty.pem", "")] {
            let path = write_temp_file(name, contents);
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
        let ca = write_temp_file("junk-ca-client.pem", "not a pem file\n");
        let missing = missing_path("client.pem");
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
        let ca = write_temp_file("junk-ca-server.pem", "not a pem file\n");
        let missing = missing_path("server.pem");
        let err = match TlsServerConfig::with_mtls(&missing, &missing, &ca) {
            Ok(_) => panic!("expected an error for an mTLS CA file without certificates"),
            Err(e) => e,
        };
        let message = err.to_string();
        assert!(message.contains("No certificates found"), "{message}");
        assert!(message.contains("junk-ca-server.pem"), "{message}");
        let _ = std::fs::remove_file(&ca);
    }

    // Note: Additional tests would require test certificates
    // These should be added in integration tests
}
