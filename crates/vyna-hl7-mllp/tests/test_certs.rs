//! Test certificate generation utilities
//!
//! This module provides helpers for generating temporary TLS certificates
//! for integration testing.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Test certificates bundle
pub struct TestCerts {
    pub ca_cert_path: String,
    pub ca_key_path: String,
    pub server_cert_path: String,
    pub server_key_path: String,
    pub temp_dir: PathBuf,
}

/// Test certificates bundle with client cert (for mTLS)
pub struct TestCertsWithClient {
    pub ca_cert_path: String,
    #[allow(dead_code)]
    pub ca_key_path: String,
    pub server_cert_path: String,
    pub server_key_path: String,
    pub client_cert_path: String,
    pub client_key_path: String,
    pub temp_dir: PathBuf,
}

impl TestCerts {
    /// Clean up temporary certificate files
    pub fn cleanup(self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

impl TestCertsWithClient {
    /// Clean up temporary certificate files
    pub fn cleanup(self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

/// Generate test certificates (CA + server cert)
pub async fn generate_test_certs() -> TestCerts {
    // Create temporary directory
    let temp_dir =
        std::env::temp_dir().join(format!("vyna_hl7_test_certs_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    let ca_key_path = temp_dir.join("ca-key.pem");
    let ca_cert_path = temp_dir.join("ca-cert.pem");
    let server_key_path = temp_dir.join("server-key.pem");
    let server_cert_path = temp_dir.join("server-cert.pem");
    let server_csr_path = temp_dir.join("server.csr");
    let server_ext_path = temp_dir.join("server-ext.cnf");

    // Generate CA key
    run_openssl(&["genrsa", "-out", ca_key_path.to_str().unwrap(), "2048"]);

    // Generate CA certificate (X.509 v3 with extensions)
    run_openssl(&[
        "req",
        "-new",
        "-x509",
        "-days",
        "365",
        "-key",
        ca_key_path.to_str().unwrap(),
        "-out",
        ca_cert_path.to_str().unwrap(),
        "-subj",
        "/CN=Test CA",
        "-extensions",
        "v3_ca",
    ]);

    // Generate server key
    run_openssl(&["genrsa", "-out", server_key_path.to_str().unwrap(), "2048"]);

    // Generate server CSR
    run_openssl(&[
        "req",
        "-new",
        "-key",
        server_key_path.to_str().unwrap(),
        "-out",
        server_csr_path.to_str().unwrap(),
        "-subj",
        "/CN=localhost",
    ]);

    // Create X.509 v3 extension file for server certificate
    let ext_config = r#"[v3_req]
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
"#;
    fs::write(&server_ext_path, ext_config).expect("Failed to write extension config");

    // Sign server certificate with CA using v3 extensions
    run_openssl(&[
        "x509",
        "-req",
        "-days",
        "365",
        "-in",
        server_csr_path.to_str().unwrap(),
        "-CA",
        ca_cert_path.to_str().unwrap(),
        "-CAkey",
        ca_key_path.to_str().unwrap(),
        "-CAcreateserial",
        "-out",
        server_cert_path.to_str().unwrap(),
        "-extfile",
        server_ext_path.to_str().unwrap(),
        "-extensions",
        "v3_req",
    ]);

    TestCerts {
        ca_cert_path: ca_cert_path.to_str().unwrap().to_string(),
        ca_key_path: ca_key_path.to_str().unwrap().to_string(),
        server_cert_path: server_cert_path.to_str().unwrap().to_string(),
        server_key_path: server_key_path.to_str().unwrap().to_string(),
        temp_dir,
    }
}

/// Generate test certificates including client cert (for mTLS testing)
pub async fn generate_test_certs_with_client() -> TestCertsWithClient {
    // First generate CA and server certs
    let base_certs = generate_test_certs().await;

    let client_key_path = base_certs.temp_dir.join("client-key.pem");
    let client_cert_path = base_certs.temp_dir.join("client-cert.pem");
    let client_csr_path = base_certs.temp_dir.join("client.csr");
    let client_ext_path = base_certs.temp_dir.join("client-ext.cnf");

    // Generate client key
    run_openssl(&["genrsa", "-out", client_key_path.to_str().unwrap(), "2048"]);

    // Generate client CSR
    run_openssl(&[
        "req",
        "-new",
        "-key",
        client_key_path.to_str().unwrap(),
        "-out",
        client_csr_path.to_str().unwrap(),
        "-subj",
        "/CN=test-client",
    ]);

    // Create X.509 v3 extension file for client certificate
    let ext_config = r#"[v3_req]
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = clientAuth
"#;
    fs::write(&client_ext_path, ext_config).expect("Failed to write client extension config");

    // Sign client certificate with CA using v3 extensions
    run_openssl(&[
        "x509",
        "-req",
        "-days",
        "365",
        "-in",
        client_csr_path.to_str().unwrap(),
        "-CA",
        &base_certs.ca_cert_path,
        "-CAkey",
        &base_certs.ca_key_path,
        "-CAcreateserial",
        "-out",
        client_cert_path.to_str().unwrap(),
        "-extfile",
        client_ext_path.to_str().unwrap(),
        "-extensions",
        "v3_req",
    ]);

    TestCertsWithClient {
        ca_cert_path: base_certs.ca_cert_path,
        ca_key_path: base_certs.ca_key_path,
        server_cert_path: base_certs.server_cert_path,
        server_key_path: base_certs.server_key_path,
        client_cert_path: client_cert_path.to_str().unwrap().to_string(),
        client_key_path: client_key_path.to_str().unwrap().to_string(),
        temp_dir: base_certs.temp_dir,
    }
}

/// Run openssl command
fn run_openssl(args: &[&str]) {
    let output = Command::new("openssl")
        .args(args)
        .output()
        .expect("Failed to run openssl command. Make sure openssl is installed.");

    if !output.status.success() {
        panic!(
            "OpenSSL command failed: {}\nStderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_test_certs() {
        let certs = generate_test_certs().await;

        // Verify all certificate files exist
        assert!(
            std::path::Path::new(&certs.ca_cert_path).exists(),
            "CA cert should exist"
        );
        assert!(
            std::path::Path::new(&certs.ca_key_path).exists(),
            "CA key should exist"
        );
        assert!(
            std::path::Path::new(&certs.server_cert_path).exists(),
            "Server cert should exist"
        );
        assert!(
            std::path::Path::new(&certs.server_key_path).exists(),
            "Server key should exist"
        );

        // Cleanup
        certs.cleanup();
    }

    #[tokio::test]
    async fn test_generate_test_certs_with_client() {
        let certs = generate_test_certs_with_client().await;

        // Verify all certificate files exist including client
        assert!(
            std::path::Path::new(&certs.ca_cert_path).exists(),
            "CA cert should exist"
        );
        assert!(
            std::path::Path::new(&certs.server_cert_path).exists(),
            "Server cert should exist"
        );
        assert!(
            std::path::Path::new(&certs.client_cert_path).exists(),
            "Client cert should exist"
        );
        assert!(
            std::path::Path::new(&certs.client_key_path).exists(),
            "Client key should exist"
        );

        // Cleanup
        certs.cleanup();
    }
}
