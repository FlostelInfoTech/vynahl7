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
use rustls::pki_types::{PrivateKeyDer, pem::PemObject};
#[cfg(feature = "tls")]
use rustls::{ClientConfig, ConfigBuilder, RootCertStore, ServerConfig, WantsVerifier};
// Re-exported so downstream callers can name the provider and peer-cert
// types without pinning their own rustls version to this crate's.
#[cfg(feature = "tls")]
pub use rustls::crypto::CryptoProvider;
#[cfg(feature = "tls")]
pub use rustls::pki_types::CertificateDer;
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

/// Parse PEM certificates from in-memory bytes (same DER-out contract as
/// [`load_certs`], without the file read — for material sourced from a
/// database row, KMS, or secret manager rather than disk).
#[cfg(feature = "tls")]
fn parse_certs_pem(pem: &[u8], what: &str) -> Result<Vec<CertificateDer<'static>>> {
    CertificateDer::pem_slice_iter(pem)
        .map(|item| {
            item.map(|cert| cert.into_owned())
                .map_err(|e| Error::Network(format!("Failed to parse {what}: {e}")))
        })
        .collect()
}

/// Parse the first PEM private key (PKCS#8, PKCS#1, or SEC1) from
/// in-memory bytes. Byte-slice twin of [`load_private_key`].
#[cfg(feature = "tls")]
fn parse_private_key_pem(pem: &[u8]) -> Result<PrivateKeyDer<'static>> {
    PrivateKeyDer::pem_slice_iter(pem)
        .next()
        .transpose()
        .map_err(|e| Error::Network(format!("Failed to parse private key: {e}")))?
        .map(|k| k.clone_key())
        .ok_or_else(|| Error::Network("No private key found in PEM".to_string()))
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

/// Begin a client configuration on an explicit provider.
///
/// Equivalent to `ClientConfig::builder()`, which is itself
/// `builder_with_provider(..).with_protocol_versions(DEFAULT_VERSIONS).unwrap()`,
/// except that the provider is chosen instead of inferred. The `expect` is
/// unreachable for the same reason rustls' own is: it fires only if the provider
/// supports none of the default protocol versions.
#[cfg(feature = "tls")]
pub(crate) fn client_config_builder_with(
    provider: Arc<CryptoProvider>,
) -> ConfigBuilder<ClientConfig, WantsVerifier> {
    ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("the rustls provider supports none of the default TLS protocol versions")
}

/// Begin a server configuration on an explicit provider. See
/// [`client_config_builder_with`].
#[cfg(feature = "tls")]
pub(crate) fn server_config_builder_with(
    provider: Arc<CryptoProvider>,
) -> ConfigBuilder<ServerConfig, WantsVerifier> {
    ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("the rustls provider supports none of the default TLS protocol versions")
}

/// Resolve the provider for a build: the explicit one when given,
/// otherwise [`crypto_provider`].
#[cfg(feature = "tls")]
pub(crate) fn resolve_provider(provider: Option<Arc<CryptoProvider>>) -> Arc<CryptoProvider> {
    provider.unwrap_or_else(crypto_provider)
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
        Self::build_with_provider(None)
    }

    /// [`new`](Self::new) on an explicit provider (`None` resolves via
    /// [`resolve_provider`]).
    fn build_with_provider(provider: Option<Arc<CryptoProvider>>) -> Result<Self> {
        let mut root_store = RootCertStore::empty();

        // Add system root certificates
        for cert in webpki_roots::TLS_SERVER_ROOTS.iter() {
            root_store.roots.push(cert.clone());
        }

        let config = client_config_builder_with(resolve_provider(provider))
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
        Self::build_with_ca_cert(ca_cert_path.as_ref(), None)
    }

    /// [`with_ca_cert`](Self::with_ca_cert) on an explicit provider
    /// (`None` resolves via [`resolve_provider`]).
    fn build_with_ca_cert(
        ca_cert_path: &Path,
        provider: Option<Arc<CryptoProvider>>,
    ) -> Result<Self> {
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

        let config = client_config_builder_with(resolve_provider(provider))
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
    /// use vyna_hl7_mllp::tls::TlsClientConfig;
    ///
    /// let ca_pem = std::fs::read("ca-cert.pem").unwrap();
    /// let config = TlsClientConfig::with_ca_cert_pem(&ca_pem).unwrap();
    /// # }
    /// ```
    pub fn with_ca_cert_pem(ca_pem: &[u8]) -> Result<Self> {
        Self::build_with_ca_cert_pem(ca_pem, None)
    }

    /// [`with_ca_cert_pem`](Self::with_ca_cert_pem) on an explicit provider
    /// (`None` resolves via [`resolve_provider`]).
    fn build_with_ca_cert_pem(
        ca_pem: &[u8],
        provider: Option<Arc<CryptoProvider>>,
    ) -> Result<Self> {
        let mut root_store = RootCertStore::empty();

        let ca_certs = parse_certs_pem(ca_pem, "CA certificate")?;
        if ca_certs.is_empty() {
            return Err(Error::Network(
                "No certificates found in CA certificate PEM".to_string(),
            ));
        }

        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| Error::Network(format!("Failed to add CA certificate: {e}")))?;
        }

        let config = client_config_builder_with(resolve_provider(provider))
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
        Self::build_with_mtls(
            ca_cert_path.as_ref(),
            client_cert_path.as_ref(),
            client_key_path.as_ref(),
            None,
        )
    }

    /// [`with_mtls`](Self::with_mtls) on an explicit provider (`None`
    /// resolves via [`resolve_provider`]).
    fn build_with_mtls(
        ca_cert_path: &Path,
        client_cert_path: &Path,
        client_key_path: &Path,
        provider: Option<Arc<CryptoProvider>>,
    ) -> Result<Self> {
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

        let config = client_config_builder_with(resolve_provider(provider))
            .with_root_certificates(root_store)
            .with_client_auth_cert(client_certs, client_key)
            .map_err(|e| Error::Network(format!("Failed to configure client auth: {}", e)))?;

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
    /// use vyna_hl7_mllp::tls::TlsClientConfig;
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
        Self::build_with_mtls_pem(ca_pem, client_cert_pem, client_key_pem, None)
    }

    /// [`with_mtls_pem`](Self::with_mtls_pem) on an explicit provider
    /// (`None` resolves via [`resolve_provider`]).
    fn build_with_mtls_pem(
        ca_pem: &[u8],
        client_cert_pem: &[u8],
        client_key_pem: &[u8],
        provider: Option<Arc<CryptoProvider>>,
    ) -> Result<Self> {
        let mut root_store = RootCertStore::empty();

        let ca_certs = parse_certs_pem(ca_pem, "CA certificate")?;
        if ca_certs.is_empty() {
            return Err(Error::Network(
                "No certificates found in CA certificate PEM".to_string(),
            ));
        }

        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| Error::Network(format!("Failed to add CA certificate: {e}")))?;
        }

        let client_certs: Vec<CertificateDer> =
            parse_certs_pem(client_cert_pem, "client certificate")?;

        let client_key = parse_private_key_pem(client_key_pem)?;

        let config = client_config_builder_with(resolve_provider(provider))
            .with_root_certificates(root_store)
            .with_client_auth_cert(client_certs, client_key)
            .map_err(|e| Error::Network(format!("Failed to configure client auth: {e}")))?;

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Start a builder for configurations on an explicit [`CryptoProvider`].
    ///
    /// Without [`crypto_provider`](TlsClientConfigBuilder::crypto_provider)
    /// the builder behaves exactly like the plain constructors.
    pub fn builder() -> TlsClientConfigBuilder {
        TlsClientConfigBuilder::new()
    }
}

/// Builder for [`TlsClientConfig`] with an optional explicit [`CryptoProvider`].
///
/// The plain constructors resolve the provider per [`crypto_provider`]
/// (installed process default, else `aws-lc-rs`). Use this builder when the
/// application must pin a provider per connection — a `ring` deployment, say —
/// without touching the process-global default and without risking the
/// mixed-provider panic a second `install_default` would cause.
///
/// Each terminal mirrors the same-named [`TlsClientConfig`] constructor.
#[cfg(feature = "tls")]
#[derive(Debug, Default)]
pub struct TlsClientConfigBuilder {
    provider: Option<Arc<CryptoProvider>>,
}

#[cfg(feature = "tls")]
impl TlsClientConfigBuilder {
    /// Start a builder resolving the provider per [`crypto_provider`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Pin the [`CryptoProvider`] the built configuration uses.
    ///
    /// The provider must be usable for both TLS protocol versions and
    /// certificate verification; either rustls default
    /// (`aws_lc_rs::default_provider()` or `ring::default_provider()`)
    /// qualifies.
    pub fn crypto_provider(mut self, provider: Arc<CryptoProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Build with system root certificates. Mirrors [`TlsClientConfig::new`].
    pub fn build(self) -> Result<TlsClientConfig> {
        TlsClientConfig::build_with_provider(self.provider)
    }

    /// Build with a custom root CA. Mirrors [`TlsClientConfig::with_ca_cert`].
    pub fn build_with_ca_cert(self, ca_cert_path: impl AsRef<Path>) -> Result<TlsClientConfig> {
        TlsClientConfig::build_with_ca_cert(ca_cert_path.as_ref(), self.provider)
    }

    /// Build with a custom root CA from PEM bytes.
    /// Mirrors [`TlsClientConfig::with_ca_cert_pem`].
    pub fn build_with_ca_cert_pem(self, ca_pem: &[u8]) -> Result<TlsClientConfig> {
        TlsClientConfig::build_with_ca_cert_pem(ca_pem, self.provider)
    }

    /// Build with mutual TLS. Mirrors [`TlsClientConfig::with_mtls`].
    pub fn build_with_mtls(
        self,
        ca_cert_path: impl AsRef<Path>,
        client_cert_path: impl AsRef<Path>,
        client_key_path: impl AsRef<Path>,
    ) -> Result<TlsClientConfig> {
        TlsClientConfig::build_with_mtls(
            ca_cert_path.as_ref(),
            client_cert_path.as_ref(),
            client_key_path.as_ref(),
            self.provider,
        )
    }

    /// Build with mutual TLS from PEM bytes.
    /// Mirrors [`TlsClientConfig::with_mtls_pem`].
    pub fn build_with_mtls_pem(
        self,
        ca_pem: &[u8],
        client_cert_pem: &[u8],
        client_key_pem: &[u8],
    ) -> Result<TlsClientConfig> {
        TlsClientConfig::build_with_mtls_pem(ca_pem, client_cert_pem, client_key_pem, self.provider)
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
    pub fn new(cert_path: impl AsRef<Path>, key_path: impl AsRef<Path>) -> Result<Self> {
        Self::build_new(cert_path.as_ref(), key_path.as_ref(), None)
    }

    /// [`new`](Self::new) on an explicit provider (`None` resolves via
    /// [`resolve_provider`]).
    fn build_new(
        cert_path: &Path,
        key_path: &Path,
        provider: Option<Arc<CryptoProvider>>,
    ) -> Result<Self> {
        // Load server certificate
        let certs: Vec<CertificateDer> = load_certs(cert_path, "server certificate")?;

        // Load server private key
        let key = load_private_key(key_path)?;

        let config = server_config_builder_with(resolve_provider(provider))
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| Error::Network(format!("Failed to configure server TLS: {}", e)))?;

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
    /// use vyna_hl7_mllp::tls::TlsServerConfig;
    ///
    /// let cert_pem = std::fs::read("server-cert.pem").unwrap();
    /// let key_pem = std::fs::read("server-key.pem").unwrap();
    /// let config = TlsServerConfig::new_pem(&cert_pem, &key_pem).unwrap();
    /// # }
    /// ```
    pub fn new_pem(cert_pem: &[u8], key_pem: &[u8]) -> Result<Self> {
        Self::build_new_pem(cert_pem, key_pem, None)
    }

    /// [`new_pem`](Self::new_pem) on an explicit provider (`None` resolves
    /// via [`resolve_provider`]).
    fn build_new_pem(
        cert_pem: &[u8],
        key_pem: &[u8],
        provider: Option<Arc<CryptoProvider>>,
    ) -> Result<Self> {
        let certs: Vec<CertificateDer> = parse_certs_pem(cert_pem, "server certificate")?;

        let key = parse_private_key_pem(key_pem)?;

        let config = server_config_builder_with(resolve_provider(provider))
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| Error::Network(format!("Failed to configure server TLS: {e}")))?;

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
        Self::build_with_mtls(
            cert_path.as_ref(),
            key_path.as_ref(),
            ca_cert_path.as_ref(),
            None,
        )
    }

    /// [`with_mtls`](Self::with_mtls) on an explicit provider (`None`
    /// resolves via [`resolve_provider`]).
    fn build_with_mtls(
        cert_path: &Path,
        key_path: &Path,
        ca_cert_path: &Path,
        provider: Option<Arc<CryptoProvider>>,
    ) -> Result<Self> {
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

        // The verifier and the server config share one provider so the
        // signature schemes always agree.
        let provider = resolve_provider(provider);
        let client_cert_verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(client_auth_roots),
            provider.clone(),
        )
        .build()
        .map_err(|e| Error::Network(format!("Failed to build client verifier: {}", e)))?;

        let config = server_config_builder_with(provider)
            .with_client_cert_verifier(client_cert_verifier)
            .with_single_cert(certs, key)
            .map_err(|e| Error::Network(format!("Failed to configure server TLS: {}", e)))?;

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
    /// use vyna_hl7_mllp::tls::TlsServerConfig;
    ///
    /// let cert_pem = std::fs::read("server-cert.pem").unwrap();
    /// let key_pem = std::fs::read("server-key.pem").unwrap();
    /// let ca_pem = std::fs::read("ca-cert.pem").unwrap();
    /// let config =
    ///     TlsServerConfig::with_mtls_pem(&cert_pem, &key_pem, &ca_pem).unwrap();
    /// # }
    /// ```
    pub fn with_mtls_pem(cert_pem: &[u8], key_pem: &[u8], ca_pem: &[u8]) -> Result<Self> {
        Self::build_with_mtls_pem(cert_pem, key_pem, ca_pem, None)
    }

    /// [`with_mtls_pem`](Self::with_mtls_pem) on an explicit provider
    /// (`None` resolves via [`resolve_provider`]).
    fn build_with_mtls_pem(
        cert_pem: &[u8],
        key_pem: &[u8],
        ca_pem: &[u8],
        provider: Option<Arc<CryptoProvider>>,
    ) -> Result<Self> {
        let mut client_auth_roots = RootCertStore::empty();
        let ca_certs = parse_certs_pem(ca_pem, "CA certificate")?;
        if ca_certs.is_empty() {
            return Err(Error::Network(
                "No certificates found in CA certificate PEM".to_string(),
            ));
        }

        for cert in ca_certs {
            client_auth_roots
                .add(cert)
                .map_err(|e| Error::Network(format!("Failed to add CA certificate: {e}")))?;
        }

        let certs: Vec<CertificateDer> = parse_certs_pem(cert_pem, "server certificate")?;

        let key = parse_private_key_pem(key_pem)?;

        // The verifier and the server config share one provider so the
        // signature schemes always agree.
        let provider = resolve_provider(provider);
        let client_cert_verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(client_auth_roots),
            provider.clone(),
        )
        .build()
        .map_err(|e| Error::Network(format!("Failed to build client verifier: {e}")))?;

        let config = server_config_builder_with(provider)
            .with_client_cert_verifier(client_cert_verifier)
            .with_single_cert(certs, key)
            .map_err(|e| Error::Network(format!("Failed to configure server TLS: {e}")))?;

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Start a builder for configurations on an explicit [`CryptoProvider`].
    ///
    /// Without [`crypto_provider`](TlsServerConfigBuilder::crypto_provider)
    /// the builder behaves exactly like the plain constructors.
    pub fn builder() -> TlsServerConfigBuilder {
        TlsServerConfigBuilder::new()
    }
}

/// Builder for [`TlsServerConfig`] with an optional explicit [`CryptoProvider`].
///
/// The plain constructors resolve the provider per [`crypto_provider`]
/// (installed process default, else `aws-lc-rs`). Use this builder when the
/// application must pin a provider per listener — a `ring` deployment, say —
/// without touching the process-global default and without risking the
/// mixed-provider panic a second `install_default` would cause.
///
/// Each terminal mirrors the same-named [`TlsServerConfig`] constructor.
#[cfg(feature = "tls")]
#[derive(Debug, Default)]
pub struct TlsServerConfigBuilder {
    provider: Option<Arc<CryptoProvider>>,
}

#[cfg(feature = "tls")]
impl TlsServerConfigBuilder {
    /// Start a builder resolving the provider per [`crypto_provider`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Pin the [`CryptoProvider`] the built configuration uses.
    ///
    /// The same provider feeds both the client-certificate verifier and
    /// the server config, so their signature schemes always agree. Either
    /// rustls default (`aws_lc_rs::default_provider()` or
    /// `ring::default_provider()`) qualifies.
    pub fn crypto_provider(mut self, provider: Arc<CryptoProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Build from cert/key files. Mirrors [`TlsServerConfig::new`].
    pub fn build(
        self,
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
    ) -> Result<TlsServerConfig> {
        TlsServerConfig::build_new(cert_path.as_ref(), key_path.as_ref(), self.provider)
    }

    /// Build from PEM bytes. Mirrors [`TlsServerConfig::new_pem`].
    pub fn build_pem(self, cert_pem: &[u8], key_pem: &[u8]) -> Result<TlsServerConfig> {
        TlsServerConfig::build_new_pem(cert_pem, key_pem, self.provider)
    }

    /// Build with mutual TLS. Mirrors [`TlsServerConfig::with_mtls`].
    pub fn build_with_mtls(
        self,
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
        ca_cert_path: impl AsRef<Path>,
    ) -> Result<TlsServerConfig> {
        TlsServerConfig::build_with_mtls(
            cert_path.as_ref(),
            key_path.as_ref(),
            ca_cert_path.as_ref(),
            self.provider,
        )
    }

    /// Build with mutual TLS from PEM bytes.
    /// Mirrors [`TlsServerConfig::with_mtls_pem`].
    pub fn build_with_mtls_pem(
        self,
        cert_pem: &[u8],
        key_pem: &[u8],
        ca_pem: &[u8],
    ) -> Result<TlsServerConfig> {
        TlsServerConfig::build_with_mtls_pem(cert_pem, key_pem, ca_pem, self.provider)
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

    // Static self-signed fixture (CN=pem-ctor-test, 10-year expiry,
    // generated once with openssl; test-only material, never trusted
    // anywhere). One cert doubles as CA, server, and client identity:
    // config construction only parses and checks key/cert consistency.
    const FIXTURE_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\nMIIDETCCAfmgAwIBAgIUBadJx+R2ThDFVEs2iULZY3ep+zAwDQYJKoZIhvcNAQEL\nBQAwGDEWMBQGA1UEAwwNcGVtLWN0b3ItdGVzdDAeFw0yNjEwMTUwNjEyMjRaFw0z\nNjEwMTIwNjEyMjRaMBgxFjAUBgNVBAMMDXBlbS1jdG9yLXRlc3QwggEiMA0GCSqG\nSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDNpR1oubUSO9nLf8Utipi3e+SO0HYEgn2J\nKgUDcSM/PhMRRZI7/hwZh7gZ0Gx+cNwCggyQvLxNo539M131Q9wiKP6aPz8QkMJa\nAyEwAZlSDaFkj4ve4EefBqIUdZAKLefCNcz4IlH/DPfRmH1iCcsc+SzSPK3o13ed\nFIhbjmM+pGQOc2kvw9h6zGJOq9Y6uYfVAcCVyn49K5gNw8jpQ1bKLE2uTwEvDy7g\n2omPybiXcWjC3aI0mg5RF6ysd4k7p8Mb5hZ4U0hmiixOp1+49tUOhSim0Dr/ylL6\nmlEjBYwspNflAQLzfo27tbGbvcFcG+4Gx5EMDrRzkmfm5za+qJmDAgMBAAGjUzBR\nMB0GA1UdDgQWBBTlaRk4stco+I8bCPcIQfL5Da8sxzAfBgNVHSMEGDAWgBTlaRk4\nstco+I8bCPcIQfL5Da8sxzAPBgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3DQEBCwUA\nA4IBAQAJynfo6bLVvBtWeVIfrh/BDBBAw/oT6S0t3hBF37IAGp0OKS+hA6C/nOOl\nJfnyC87xeXnPhRf3Cf6rhiQoZfijpphkCJ7vy3l/tjzODw1x4HSHzA6qpYG4sfXy\nYRxt8PNdWZy9/9apWW/36tBrlPGMmgLRFRQFaYB9Wt6xt/zmrPTDdWN6RrS33Nvn\n1C07xWOi4Kr+G2mnsQax66SmlDGLuWHlzNpQE/P6x11d1rPYUE/O0UWgaGrxvUu6\nb45RrM6BCsr7LswJXvjii1AfgHsqh5NaCBOkTUemQY2EhY968QW6L6Dg0R2EHDwZ\nI4Ijz1YQlVbQQYlRN7NzH7Pvg2JT\n-----END CERTIFICATE-----\n";
    const FIXTURE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDNpR1oubUSO9nL\nf8Utipi3e+SO0HYEgn2JKgUDcSM/PhMRRZI7/hwZh7gZ0Gx+cNwCggyQvLxNo539\nM131Q9wiKP6aPz8QkMJaAyEwAZlSDaFkj4ve4EefBqIUdZAKLefCNcz4IlH/DPfR\nmH1iCcsc+SzSPK3o13edFIhbjmM+pGQOc2kvw9h6zGJOq9Y6uYfVAcCVyn49K5gN\nw8jpQ1bKLE2uTwEvDy7g2omPybiXcWjC3aI0mg5RF6ysd4k7p8Mb5hZ4U0hmiixO\np1+49tUOhSim0Dr/ylL6mlEjBYwspNflAQLzfo27tbGbvcFcG+4Gx5EMDrRzkmfm\n5za+qJmDAgMBAAECggEAWQ26TRoyqBlECUSlAkJFPRwJT7FFklFiBK6uKh3p+h6r\nCi07FQDHcWTdHEyuVxkPpYRk5Z7d2q9SoQ4ID8jZwO9IxkpCSK2vhGRbkFr45tgh\nPpeNJjCww1SPhtWcUVQsy2UaBJB0OsOGY/0puvVrwHnFZ6vAUhdQ5LS6Eg/4uWyX\nXn2eu3b2wwATTjAgIvahvvydhGSuqP/pRR74bfbvXJYtIRSsxa6oDTVaSs7gvFkz\nLp2iU4jMJ/zbMBLbNEB1Q96YvhamF6FPEuic4BxMWt7ZROFLCnkwQrKhjoL1HXbC\naTFmOReGNPvwD+FaNSM1G0s6X6ZDsCHyImxfX1HeLQKBgQDrpflDwmTBU70Nh98J\nSUeYSfxZGIClk88P4/4WSioYhBjzspDVLbWQalAIoGpBrKFO/Ry5yIbp/nIWX7K8\nnx02aaF+RpUXGAieAlMQEcDFQM94/+vGikZl3RyI6FopFSFyWqKeJZDWms1XYzb1\nc8nKp9NcSxRPbTOPQjGDz6mMxQKBgQDfZ8mEo9fXxZxcRkjCQNkgNNVnHpd/Ndjd\nbntoAZ6Kv1kvPjOXFhadQERSRY49IHUCVb7zdiLCP06nEJAARi8iKM8mRDshfwEm\nfaexKQ1QHLLNiwYQsBdIrTLpWtg9QKNJ+ZcovQt+O6o73oS65l09a+UfZkA7KH9c\nqS3jEY0BpwKBgQCJuIDLxyrXijctV0FUzg0XxfXDpHxbSN11LgxrvoCHv3xNwQjA\nbmx0sI2B3S78nwXPgkJtmfKvgtPum5N8hYmvKWEGyf2Md7Fhl34wIBAHuG8JCpkj\neeDW27k3koRPClh32ty7jLzvj51++y89mKRQ2nC+uepu2SWPDmKd5Jn0+QKBgE40\nqOjT6lVQigILYKEPznJlzGbC4MnZccdiS7UHDBcKERBmt+ymANifjMs4Kijf2dYt\naiEUM+rH+qKlO6O87fcJTirZK9H4AdEVn8R22IA466bkigUBf8UjTHVAJf6bSA43\nX2EVfJZ+EIPQy6oZxn4f7R9mzglnjd5EIxogJLp1AoGBAKs37a8A3zrFCxnIJPtF\nYl5hOtbYfuwL1+LPhmd0+FSJerEyIjMnrBht+QFJnuC06k8b63G5HSFQdreYpVh1\nc3yb+Iwb2lhW90c6u9LLakmClWzTIo1+SwS8WSKj+HNjuO4tsZOfn8rwKMS64qpP\n6A73srO5EtsGCcggPenQtouC\n-----END PRIVATE KEY-----\n";
    const FIXTURE_OTHER_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQDFqfEG+m9yBZph\nDruDhqrOa8syO0WutFZZBKbaNFT8qnFYwJlcYsJ8SPKs4IhgXtV2XGm0TPd0lTIh\nVueP2FkBwfqKNpMuWDbB0eobXlSeYlzWJ88C5ZJBtCZCMOnsNbwBVQEknrlriNPJ\nclyFRDgRIKWJCAu3GN1OHRdbRxWDzlVW36iR1s+bmxB6B1X3tpHWFYk63AMAVfNA\nFgYdmFdhFQBu8ZLYkxK8PJebo5qc4a/zI6arw402ZbAkUZw8UNLqFpUr1hBaduGm\nqzK2sy3VQTYV9x5Lg0iQFL/itiR+yajIDqm9G8A2OBD0XGrkkgALrym7yN4SFEop\nX1uxVuupAgMBAAECggEAATt38mtMW0GEBRmwUxhEyUz1MLA+mMh1aNfmS+2CPdtU\nP/wslNOaPu93kZIM+19B8cOABbb7/MfGobZ6+lR0CBzDjE5a7utvYV6a8LVE/c1m\nL3SCxCBWaLAA+tJoKp9Jm+XjuI1ryuxBRL1Fk6CRoiFPalYuJEg14dt74uGvmT//\nwKLnxl6+PpacH/XW7cSj0Qr8XlGLBLx7yNsezTlKUrKXPeFjOik/XK3F+WFWmIV9\n7934HwqLp7hkrWVoLhXzEc5rXzARy4vb7SJU3WJv289MTLEKw9+nCvpyAdS3im2C\neLP0AL7PsH9AuYhc/P9ZXTooAU8+JeE4xLHlqVhM8wKBgQD/JIAVDPtij+j2ZY3W\nnxcgES3wlxCb6LH6k8IDszlKbccIHN2jHyvPZv1B564jvhZuLe7ZCAe/xb0r5v/R\naIEoz5gBgejGiW6QclGOOrRaBhBy6arvfed9HH0Qd31doMVCS07GZi7IVDIBhTIr\ntz3xP1YGTIwF2kvDBJR2vPJQIwKBgQDGU/368CXUQfjsaFI3Yjrm2K3hCDkPpzOy\n/FeQzemdTQGUY75JtCelcT9RnSuEb3gGGg9SPkLWwHcspYWVW/CaXXnaGnTL3DIR\nfvVcyrdmhCHPWS8LSH9G6bmzYs6bEIpmg4kQVwkyxVemoCKnSsyBhzRjWmGXu/Ji\nU7fLV1orwwKBgBSzHGUY0YhUjpz27kP8Uj76xdRgHSvY/B+NP09Xig1czKn+1Yh3\nbfF8tI1VQ46IzH8R4O/gs+g+HbPjINwXc2shNH7i0ETTP5IehDlaz+kB1vHLvV+C\nwnYnCop8d0JSWc6gF3sHLUJ5ejkfWkurqS3aaptGOJH8/OqMg5MCj1IrAoGAPAlr\nBg6ydOmvr+j4knly+TS4ljmGBH4KAXGwFE6xHIQWKVR0Yx1dLxD0KQGaDdIslDRJ\nVQ0x8I58xRAqOweE80W+wE5n8tpqca9sGTXohOkUzw7hVdoRZWcVZv9PKdO9AcFp\nuX+PckYY/YDWXKwg8kj6i+TtTCCCBE/xgGWmDz8CgYAq2wJWwTXJSI6kbBaqkIOk\nRZEMn2f81FHLghXRIUZ1xIJIniW1LrejWz6uFn5nckwad3y1ZXAgcNdr73lHOI5r\nsLgD1KmYUZlIeCcaW5uk162VcOM8cf9nIRZCrUEHg1t40ruqA+iryjtIqFFHw5EW\nhd6+cELw2ZXCriD/uuHnPQ==\n-----END PRIVATE KEY-----\n";

    #[test]
    fn pem_ca_cert_rejects_bytes_without_certificates() {
        for pem in ["not a pem file\n", ""] {
            let err = match TlsClientConfig::with_ca_cert_pem(pem.as_bytes()) {
                Ok(_) => panic!("expected an error for CA bytes without certificates"),
                Err(e) => e,
            };
            assert!(
                err.to_string()
                    .contains("No certificates found in CA certificate PEM"),
                "{}",
                err.to_string()
            );
        }
    }

    #[test]
    fn pem_client_mtls_rejects_empty_ca() {
        let err = match TlsClientConfig::with_mtls_pem(
            b"not a pem file\n",
            FIXTURE_CERT_PEM.as_bytes(),
            FIXTURE_KEY_PEM.as_bytes(),
        ) {
            Ok(_) => panic!("expected an error for an mTLS CA without certificates"),
            Err(e) => e,
        };
        assert!(
            err.to_string()
                .contains("No certificates found in CA certificate PEM"),
            "{}",
            err.to_string()
        );
    }

    #[test]
    fn pem_server_mtls_rejects_empty_ca() {
        let err = match TlsServerConfig::with_mtls_pem(
            FIXTURE_CERT_PEM.as_bytes(),
            FIXTURE_KEY_PEM.as_bytes(),
            b"not a pem file\n",
        ) {
            Ok(_) => panic!("expected an error for an mTLS CA without certificates"),
            Err(e) => e,
        };
        assert!(
            err.to_string()
                .contains("No certificates found in CA certificate PEM"),
            "{}",
            err.to_string()
        );
    }

    #[test]
    fn pem_constructors_accept_valid_material() {
        let cert = FIXTURE_CERT_PEM.as_bytes();
        let key = FIXTURE_KEY_PEM.as_bytes();
        assert!(TlsClientConfig::with_ca_cert_pem(cert).is_ok());
        assert!(TlsClientConfig::with_mtls_pem(cert, cert, key).is_ok());
        assert!(TlsServerConfig::new_pem(cert, key).is_ok());
        assert!(TlsServerConfig::with_mtls_pem(cert, key, cert).is_ok());
    }

    #[test]
    fn pem_server_rejects_mismatched_key() {
        let err = match TlsServerConfig::new_pem(
            FIXTURE_CERT_PEM.as_bytes(),
            FIXTURE_OTHER_KEY_PEM.as_bytes(),
        ) {
            Ok(_) => panic!("expected an error for a key that does not match the cert"),
            Err(e) => e,
        };
        assert!(
            err.to_string().contains("Failed to configure server TLS"),
            "{}",
            err.to_string()
        );
    }

    #[test]
    fn pem_client_mtls_rejects_junk_key() {
        let err = match TlsClientConfig::with_mtls_pem(
            FIXTURE_CERT_PEM.as_bytes(),
            FIXTURE_CERT_PEM.as_bytes(),
            b"not a pem file\n",
        ) {
            Ok(_) => panic!("expected an error for a client key without key material"),
            Err(e) => e,
        };
        assert!(
            err.to_string().contains("No private key found in PEM"),
            "{}",
            err.to_string()
        );
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

        let cert = FIXTURE_CERT_PEM.as_bytes();
        let key = FIXTURE_KEY_PEM.as_bytes();
        assert!(TlsClientConfig::new().is_ok());
        assert!(TlsClientConfig::with_ca_cert_pem(cert).is_ok());
        assert!(TlsClientConfig::with_mtls_pem(cert, cert, key).is_ok());
        assert!(TlsServerConfig::new_pem(cert, key).is_ok());
        assert!(TlsServerConfig::with_mtls_pem(cert, key, cert).is_ok());

        // The path-taking twins, through temp files.
        let cert_path = write_temp_file("dual-cert.pem", FIXTURE_CERT_PEM);
        let key_path = write_temp_file("dual-key.pem", FIXTURE_KEY_PEM);
        assert!(TlsClientConfig::with_ca_cert(&cert_path).is_ok());
        assert!(TlsClientConfig::with_mtls(&cert_path, &cert_path, &key_path).is_ok());
        assert!(TlsServerConfig::new(&cert_path, &key_path).is_ok());
        assert!(TlsServerConfig::with_mtls(&cert_path, &key_path, &cert_path).is_ok());
        let _ = std::fs::remove_file(&cert_path);
        let _ = std::fs::remove_file(&key_path);
    }

    /// ENH-004: every builder terminal accepts an explicit provider.
    ///
    /// Runs under dual providers (`aws-lc-rs` + `ring`, see above) with no
    /// process default installed: the explicit `ring` provider must carry
    /// every terminal without inference and without `install_default`.
    /// Builder-without-provider must match the plain constructors.
    #[test]
    fn tls_builders_accept_explicit_provider() {
        let ring = Arc::new(rustls::crypto::ring::default_provider());
        let cert = FIXTURE_CERT_PEM.as_bytes();
        let key = FIXTURE_KEY_PEM.as_bytes();

        assert!(
            TlsClientConfig::builder()
                .crypto_provider(ring.clone())
                .build()
                .is_ok()
        );
        assert!(
            TlsClientConfig::builder()
                .crypto_provider(ring.clone())
                .build_with_ca_cert_pem(cert)
                .is_ok()
        );
        assert!(
            TlsClientConfig::builder()
                .crypto_provider(ring.clone())
                .build_with_mtls_pem(cert, cert, key)
                .is_ok()
        );
        assert!(
            TlsServerConfig::builder()
                .crypto_provider(ring.clone())
                .build_pem(cert, key)
                .is_ok()
        );
        assert!(
            TlsServerConfig::builder()
                .crypto_provider(ring.clone())
                .build_with_mtls_pem(cert, key, cert)
                .is_ok()
        );

        let cert_path = write_temp_file("provider-cert.pem", FIXTURE_CERT_PEM);
        let key_path = write_temp_file("provider-key.pem", FIXTURE_KEY_PEM);
        assert!(
            TlsClientConfig::builder()
                .crypto_provider(ring.clone())
                .build_with_ca_cert(&cert_path)
                .is_ok()
        );
        assert!(
            TlsClientConfig::builder()
                .crypto_provider(ring.clone())
                .build_with_mtls(&cert_path, &cert_path, &key_path)
                .is_ok()
        );
        assert!(
            TlsServerConfig::builder()
                .crypto_provider(ring.clone())
                .build(&cert_path, &key_path)
                .is_ok()
        );
        assert!(
            TlsServerConfig::builder()
                .crypto_provider(ring)
                .build_with_mtls(&cert_path, &key_path, &cert_path)
                .is_ok()
        );
        let _ = std::fs::remove_file(&cert_path);
        let _ = std::fs::remove_file(&key_path);

        // No explicit provider: identical to the plain constructors,
        // including the same rejection of junk material.
        assert!(TlsClientConfig::builder().build().is_ok());
        assert!(
            TlsClientConfig::builder()
                .build_with_ca_cert_pem(b"not a pem file\n")
                .is_err()
        );
        assert!(
            TlsServerConfig::builder()
                .build_pem(
                    FIXTURE_CERT_PEM.as_bytes(),
                    FIXTURE_OTHER_KEY_PEM.as_bytes()
                )
                .is_err()
        );
    }
}
