use hudsucker::certificate_authority::RcgenAuthority;
use rcgen::{CertificateParams, KeyPair, DistinguishedName, IsCa};
use std::fs;
use std::path::PathBuf;

pub fn get_ca(ca_dir: PathBuf) -> RcgenAuthority {
    let cert_path = ca_dir.join("ca.crt");
    let key_path = ca_dir.join("ca.key");

    if cert_path.exists() && key_path.exists() {
        let cert_pem = fs::read_to_string(&cert_path).expect("Failed to read CA cert");
        let key_pem = fs::read_to_string(&key_path).expect("Failed to read CA key");
        
        let key_pair = KeyPair::from_pem(&key_pem).expect("Failed to parse CA key");
        let params = CertificateParams::from_ca_cert_pem(&cert_pem)
            .expect("Failed to parse CA cert params");
        
        RcgenAuthority::new(key_pair, params, 1000)
    } else {
        if !ca_dir.exists() {
            fs::create_dir_all(&ca_dir).expect("Failed to create CA directory");
        }

        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca;
        params.distinguished_name = DistinguishedName::new();
        params.distinguished_name.push(rcgen::DnType::CommonName, "MITM Rust CA");
        params.distinguished_name.push(rcgen::DnType::OrganizationName, "MITM Rust");

        let key_pair = KeyPair::generate().expect("Failed to generate CA key");
        let cert = params.self_signed(&key_pair).expect("Failed to sign CA cert");

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        fs::write(cert_path, &cert_pem).expect("Failed to write CA cert");
        fs::write(key_path, &key_pem).expect("Failed to write CA key");

        RcgenAuthority::new(key_pair, params, 1000)
    }
}
