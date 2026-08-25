use std::sync::Arc;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use crate::ca::RootCa;

pub struct MitmEngine {
    pub ca: Arc<RootCa>,
}

impl MitmEngine {
    pub fn new(ca: Arc<RootCa>) -> Self {
        Self { ca }
    }

    pub async fn create_tls_acceptor(&self, domain: &str) -> Result<TlsAcceptor, String> {
        let (cert_pem, key_pem) = self.ca.issue_leaf_cert(domain)?;
        
        let parsed_cert = pem::parse(&cert_pem)
            .map_err(|e| format!("Failed to parse cert pem: {}", e))?;
        let cert_der = CertificateDer::from(parsed_cert.contents().to_vec());

        let parsed_key = pem::parse(&key_pem)
            .map_err(|e| format!("Failed to parse key pem: {}", e))?;
        let key_der = PrivateKeyDer::Pkcs8(parsed_key.contents().to_vec().into());

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .map_err(|e| format!("TLS ServerConfig error: {}", e))?;

        Ok(TlsAcceptor::from(Arc::new(config)))
    }
}
