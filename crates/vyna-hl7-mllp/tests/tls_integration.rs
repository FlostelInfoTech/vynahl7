//! Integration tests for MLLP TLS/mTLS functionality
//!
//! These tests verify TLS connections work correctly with the MLLP protocol.

#![cfg(all(feature = "tls", feature = "testing"))]

use vyna_hl7_core::{Field, Message, Segment};
use vyna_hl7_mllp::{
    MllpClient, MllpServer,
    testing::MockMllpServer,
    tls::{CertificateDer, TlsClientConfig, TlsServerConfig},
};

mod test_certs;

/// First certificate from a PEM file, as DER bytes.
fn first_cert_der(path: &str) -> Vec<u8> {
    use rustls::pki_types::pem::PemObject;
    let pem = std::fs::read(path).expect("read PEM fixture");
    CertificateDer::pem_slice_iter(&pem)
        .next()
        .expect("PEM file holds a certificate")
        .expect("PEM parses")
        .as_ref()
        .to_vec()
}

#[tokio::test]
async fn test_tls_basic_connection() {
    // Generate test certificates
    let certs = test_certs::generate_test_certs().await;

    // Create TLS server config
    let server_config = TlsServerConfig::new(&certs.server_cert_path, &certs.server_key_path)
        .expect("Failed to create TLS server config");

    // Start mock server with TLS
    let server = MockMllpServer::new()
        .with_tls(server_config)
        .start()
        .await
        .expect("Failed to start TLS server");

    let addr = server.url();

    // Create TLS client config
    let client_config = TlsClientConfig::with_ca_cert(&certs.ca_cert_path)
        .expect("Failed to create TLS client config");

    // Connect with TLS
    let mut client = MllpClient::connect_tls(&addr, "localhost", client_config)
        .await
        .expect("Failed to connect with TLS");

    // Create test message
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("TEST"));
    msg.segments.push(msh);

    // Send message over TLS
    let response = client
        .send_message(&msg)
        .await
        .expect("Failed to send message");

    // Verify response (server echoes by default)
    assert_eq!(msg.encode(), response.encode());

    // Cleanup
    client.close().await.expect("Failed to close client");
    server.shutdown().await.expect("Failed to shutdown server");
    certs.cleanup();
}

#[tokio::test]
async fn test_tls_custom_handler() {
    // Generate test certificates
    let certs = test_certs::generate_test_certs().await;

    // Create TLS server config
    let server_config = TlsServerConfig::new(&certs.server_cert_path, &certs.server_key_path)
        .expect("Failed to create TLS server config");

    // Start mock server with custom handler
    let server = MockMllpServer::new()
        .with_handler(|_msg| {
            let mut ack = Message::default();

            // MSH segment
            let mut msh = Segment::new("MSH");
            msh.fields.push(Field::from_value("|"));
            msh.fields.push(Field::from_value("^~\\&"));
            ack.segments.push(msh);

            // MSA segment
            let mut msa = Segment::new("MSA");
            msa.fields.push(Field::from_value("AA")); // Application Accept
            msa.fields.push(Field::from_value("TLS_TEST"));
            ack.segments.push(msa);

            Ok(ack)
        })
        .with_tls(server_config)
        .start()
        .await
        .expect("Failed to start TLS server");

    let addr = server.url();

    // Create TLS client
    let client_config = TlsClientConfig::with_ca_cert(&certs.ca_cert_path)
        .expect("Failed to create TLS client config");

    let mut client = MllpClient::connect_tls(&addr, "localhost", client_config)
        .await
        .expect("Failed to connect with TLS");

    // Send message
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msg.segments.push(msh);

    let response = client
        .send_message(&msg)
        .await
        .expect("Failed to send message");

    // Verify custom ACK
    assert_eq!(response.segments[0].id, "MSH");
    assert_eq!(response.segments[1].id, "MSA");
    assert_eq!(response.segments[1].fields[0].value(), Some("AA"));
    assert_eq!(response.segments[1].fields[1].value(), Some("TLS_TEST"));

    // Cleanup
    client.close().await.expect("Failed to close client");
    server.shutdown().await.expect("Failed to shutdown server");
    certs.cleanup();
}

#[tokio::test]
async fn test_mtls_with_client_cert() {
    // Generate test certificates including client cert
    let certs = test_certs::generate_test_certs_with_client().await;

    // Create mTLS server config (requires client certificate)
    let server_config = TlsServerConfig::with_mtls(
        &certs.server_cert_path,
        &certs.server_key_path,
        &certs.ca_cert_path,
    )
    .expect("Failed to create mTLS server config");

    // Start mock server with mTLS
    let server = MockMllpServer::new()
        .with_tls(server_config)
        .start()
        .await
        .expect("Failed to start mTLS server");

    let addr = server.url();

    // Create mTLS client config (with client certificate)
    let client_config = TlsClientConfig::with_mtls(
        &certs.ca_cert_path,
        &certs.client_cert_path,
        &certs.client_key_path,
    )
    .expect("Failed to create mTLS client config");

    // Connect with mTLS
    let mut client = MllpClient::connect_tls(&addr, "localhost", client_config)
        .await
        .expect("Failed to connect with mTLS");

    // Send message
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msg.segments.push(msh);

    let response = client
        .send_message(&msg)
        .await
        .expect("Failed to send message");

    // Verify response
    assert_eq!(msg.encode(), response.encode());

    // Cleanup
    client.close().await.expect("Failed to close client");
    server.shutdown().await.expect("Failed to shutdown server");
    certs.cleanup();
}

#[tokio::test]
async fn test_mtls_rejects_client_without_certificate() {
    // Generate test certificates (CA + server cert; no client cert)
    let certs = test_certs::generate_test_certs().await;

    // Create mTLS server config (requires client certificate)
    let server_config = TlsServerConfig::with_mtls(
        &certs.server_cert_path,
        &certs.server_key_path,
        &certs.ca_cert_path,
    )
    .expect("Failed to create mTLS server config");

    // Start mock server with mTLS
    let server = MockMllpServer::new()
        .with_tls(server_config)
        .start()
        .await
        .expect("Failed to start mTLS server");

    let addr = server.url();

    // Cert-less client: trusts the CA but presents no client certificate
    let client_config = TlsClientConfig::with_ca_cert(&certs.ca_cert_path)
        .expect("Failed to create TLS client config");

    // TLS 1.3 postpones client authentication, so connect_tls may succeed;
    // the server must reject at connect time or, at the latest, on the
    // first message exchange.
    match MllpClient::connect_tls(&addr, "localhost", client_config).await {
        Err(_) => {} // Rejected during the handshake.
        Ok(mut client) => {
            let mut msg = Message::default();
            let mut msh = Segment::new("MSH");
            msh.fields.push(Field::from_value("|"));
            msh.fields.push(Field::from_value("^~\\&"));
            msg.segments.push(msh);

            let result = client.send_message(&msg).await;
            assert!(
                result.is_err(),
                "mTLS server accepted a client without a certificate"
            );
            let _ = client.close().await;
        }
    }

    // Cleanup
    server.shutdown().await.expect("Failed to shutdown server");
    certs.cleanup();
}

#[tokio::test]
async fn test_tls_multiple_messages() {
    // Generate test certificates
    let certs = test_certs::generate_test_certs().await;

    // Create TLS server
    let server_config = TlsServerConfig::new(&certs.server_cert_path, &certs.server_key_path)
        .expect("Failed to create TLS server config");

    let server = MockMllpServer::new()
        .with_tls(server_config)
        .start()
        .await
        .expect("Failed to start TLS server");

    let addr = server.url();

    // Create TLS client
    let client_config = TlsClientConfig::with_ca_cert(&certs.ca_cert_path)
        .expect("Failed to create TLS client config");

    let mut client = MllpClient::connect_tls(&addr, "localhost", client_config)
        .await
        .expect("Failed to connect with TLS");

    // Send multiple messages over the same TLS connection
    for i in 0..5 {
        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msh.fields.push(Field::from_value("^~\\&"));
        msh.fields.push(Field::from_value(format!("TEST_{}", i)));
        msg.segments.push(msh);

        let response = client
            .send_message(&msg)
            .await
            .expect("Failed to send message");

        assert_eq!(msg.encode(), response.encode());
    }

    // Cleanup
    client.close().await.expect("Failed to close client");
    server.shutdown().await.expect("Failed to shutdown server");
    certs.cleanup();
}

#[tokio::test]
async fn test_tls_connection_refused_without_client_ca() {
    // Generate test certificates
    let certs = test_certs::generate_test_certs().await;

    // Create TLS server
    let server_config = TlsServerConfig::new(&certs.server_cert_path, &certs.server_key_path)
        .expect("Failed to create TLS server config");

    let server = MockMllpServer::new()
        .with_tls(server_config)
        .start()
        .await
        .expect("Failed to start TLS server");

    let addr = server.url();

    // Try to connect without CA certificate (should fail or require system trust)
    let client_config = TlsClientConfig::new().expect("Failed to create TLS client config");

    // This should fail because our test cert is not in system trust store
    let result = MllpClient::connect_tls(&addr, "localhost", client_config).await;

    // Connection should fail
    assert!(
        result.is_err(),
        "Connection should fail without proper CA certificate"
    );

    // Cleanup
    server.shutdown().await.expect("Failed to shutdown server");
    certs.cleanup();
}

#[tokio::test]
async fn test_tls_concurrent_connections() {
    // Generate test certificates
    let certs = test_certs::generate_test_certs().await;

    // Create TLS server
    let server_config = TlsServerConfig::new(&certs.server_cert_path, &certs.server_key_path)
        .expect("Failed to create TLS server config");

    let server = MockMllpServer::new()
        .with_tls(server_config)
        .start()
        .await
        .expect("Failed to start TLS server");

    let addr = server.url();

    // Create multiple concurrent connections
    let mut handles = vec![];

    for i in 0..5 {
        let addr = addr.clone();
        let ca_cert_path = certs.ca_cert_path.clone();

        let handle = tokio::spawn(async move {
            let client_config = TlsClientConfig::with_ca_cert(&ca_cert_path)
                .expect("Failed to create TLS client config");

            let mut client = MllpClient::connect_tls(&addr, "localhost", client_config)
                .await
                .expect("Failed to connect with TLS");

            let mut msg = Message::default();
            let mut msh = Segment::new("MSH");
            msh.fields.push(Field::from_value("|"));
            msh.fields.push(Field::from_value("^~\\&"));
            msh.fields
                .push(Field::from_value(format!("CONCURRENT_{}", i)));
            msg.segments.push(msh);

            let response = client
                .send_message(&msg)
                .await
                .expect("Failed to send message");

            assert_eq!(msg.encode(), response.encode());

            client.close().await.expect("Failed to close client");
        });

        handles.push(handle);
    }

    // Wait for all connections to complete
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // Cleanup
    server.shutdown().await.expect("Failed to shutdown server");
    certs.cleanup();
}

/// ENH-003, TLS side: over mTLS both ends report `is_tls()`, the server
/// sees the client leaf certificate and the client sees the server leaf.
#[tokio::test]
async fn test_mtls_channel_introspection() {
    let certs = test_certs::generate_test_certs_with_client().await;

    let expected_client_der = first_cert_der(&certs.client_cert_path);
    let expected_server_der = first_cert_der(&certs.server_cert_path);

    let server_config = TlsServerConfig::with_mtls(
        &certs.server_cert_path,
        &certs.server_key_path,
        &certs.ca_cert_path,
    )
    .expect("Failed to create mTLS server config");

    // Raw server (not the mock): the test needs the accepted connection.
    let server = MllpServer::bind_tls("127.0.0.1:0", server_config)
        .await
        .expect("Failed to bind TLS server");
    let addr = server.local_addr().expect("local addr").to_string();

    let server_task = tokio::spawn(async move {
        let mut conn = server.accept().await.expect("accept");
        assert!(conn.is_tls());
        let rendered = format!("{conn:?}");
        assert!(rendered.contains("tls: true"), "{rendered}");
        let peer = conn.peer_certificate().expect("mTLS client cert");
        assert_eq!(peer.as_ref(), expected_client_der.as_slice());

        // Echo one message so the client exchange completes.
        let msg = conn.receive_message().await.expect("receive");
        conn.send_message(&msg).await.expect("echo");
    });

    let client_config = TlsClientConfig::with_mtls(
        &certs.ca_cert_path,
        &certs.client_cert_path,
        &certs.client_key_path,
    )
    .expect("Failed to create mTLS client config");

    let mut client = MllpClient::connect_tls(&addr, "localhost", client_config)
        .await
        .expect("Failed to connect with mTLS");
    assert!(client.is_tls());
    let peer = client.peer_certificate().expect("server cert");
    assert_eq!(peer.as_ref(), expected_server_der.as_slice());

    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msg.segments.push(msh);

    let response = client
        .send_message(&msg)
        .await
        .expect("Failed to send message");
    assert_eq!(msg.encode(), response.encode());

    client.close().await.expect("close");
    server_task.await.expect("server task");
    certs.cleanup();
}
