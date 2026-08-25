use std::sync::Arc;
use std::time::SystemTime;
use tokio_rustls::rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use rcgen::{CertificateParams, DnType, KeyPair};
use crate::ca::CA;

pub struct MitmEngine {
    pub ca: Arc<CA>,
}

impl MitmEngine {
    pub fn new(ca: Arc<CA>) -> Self {
        Self { ca }
    }

    pub async fn create_tls_acceptor(&self, domain: &str) -> Result<TlsAcceptor, String> {
        let mut params = CertificateParams::new(vec![domain.to_string()]).map_err(|e| format!("{}", e))?;
        params.distinguished_name.push(DnType::CommonName, domain.to_string());
        params.key_usages.push(rcgen::KeyUsagePurpose::DigitalSignature);
        params.key_usages.push(rcgen::KeyUsagePurpose::KeyEncipherment);
        params.extended_key_usages.push(rcgen::ExtendedKeyUsagePurpose::ServerAuth);

        let now = SystemTime::now();
        params.not_before = time::OffsetDateTime::from(now - std::time::Duration::from_secs(86400));
        params.not_after = time::OffsetDateTime::from(now + std::time::Duration::from_secs(86400 * 365));

        let cert_key_pair = KeyPair::generate().map_err(|e| format!("{}", e))?;
        let cert = params.signed_by(&cert_key_pair, &self.ca.cert, &self.ca.key_pair).map_err(|e| format!("{}", e))?;

        let cert_der = cert.der().clone();
        let key_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(cert_key_pair.serialize_der()));

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .map_err(|e| format!("TLS ServerConfig error: {}", e))?;

        Ok(TlsAcceptor::from(Arc::new(config)))
    }
}
