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
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
#[cfg(feature = "tls")]
use rustls::{ClientConfig, RootCertStore, ServerConfig};
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
    /// use vyna_hl7_http::tls::TlsClientConfig;
    ///
    /// let config = TlsClientConfig::with_ca_cert("ca-cert.pem").unwrap();
    /// # }
    /// ```
    pub fn with_ca_cert(ca_cert_path: impl AsRef<Path>) -> Result<Self> {
        let mut root_store = RootCertStore::empty();

        // Load CA certificate
        let ca_certs = load_certs(ca_cert_path, "CA certificate")?;

        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| config_error("Failed to add CA certificate", e))?;
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
        let mut root_store = RootCertStore::empty();

        // Load CA certificate
        let ca_certs = load_certs(ca_cert_path, "CA certificate")?;

        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| config_error("Failed to add CA certificate", e))?;
        }

        // Load client certificate
        let client_certs: Vec<CertificateDer> = load_certs(client_cert_path, "client certificate")?;

        // Load client private key
        let client_key = load_private_key(client_key_path, "private key")?;

        let config = ClientConfig::builder()
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
    pub fn new(
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
    ) -> Result<Self> {
        // Load server certificate
        let certs: Vec<CertificateDer> = load_certs(cert_path, "server certificate")?;

        // Load server private key
        let key = load_private_key(key_path, "private key")?;

        let config = ServerConfig::builder()
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
        // Load CA certificate for client verification
        let mut client_auth_roots = RootCertStore::empty();
        let ca_certs = load_certs(ca_cert_path, "CA certificate")?;

        for cert in ca_certs {
            client_auth_roots
                .add(cert)
                .map_err(|e| config_error("Failed to add CA certificate", e))?;
        }

        // Load server certificate
        let certs: Vec<CertificateDer> = load_certs(cert_path, "server certificate")?;

        // Load server private key
        let key = load_private_key(key_path, "private key")?;

        let client_cert_verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(client_auth_roots))
            .build()
            .map_err(|e| config_error("Failed to build client verifier", e))?;

        let config = ServerConfig::builder()
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
        let path =
            std::env::temp_dir().join(format!("vyna_hl7_http_tls_test_{}_{}", std::process::id(), name));
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
}
