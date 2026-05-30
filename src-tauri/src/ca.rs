use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, Certificate
};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

pub struct CA {
    pub cert: Certificate,
    pub key_pair: KeyPair,
    pub cert_pem: String,
}

pub fn get_ca(ca_dir: PathBuf) -> CA {
    let cert_path = ca_dir.join("ca.crt");
    let key_path = ca_dir.join("ca.key");

    if cert_path.exists() && key_path.exists() {
        let cert_pem = fs::read_to_string(&cert_path).expect("Failed to read CA cert");
        let key_pem = fs::read_to_string(&key_path).expect("Failed to read CA key");
        
        let key_pair = KeyPair::from_pem(&key_pem).expect("Failed to parse CA key");
        let params = CertificateParams::from_ca_cert_pem(&cert_pem).expect("Failed to parse CA cert params");
        let cert = params.self_signed(&key_pair).expect("Failed to reconstruct CA cert");

        CA {
            cert,
            key_pair,
            cert_pem,
        }
    } else {
        if !ca_dir.exists() {
            fs::create_dir_all(&ca_dir).expect("Failed to create CA directory");
        }

        let mut params = CertificateParams::new(Vec::new()).expect("Failed to create cert params");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.distinguished_name = DistinguishedName::new();
        params.distinguished_name.push(DnType::CommonName, "MITM Rust CA");
        params.distinguished_name.push(DnType::OrganizationName, "MITM Rust");
        params.key_usages.push(rcgen::KeyUsagePurpose::KeyCertSign);
        params.key_usages.push(rcgen::KeyUsagePurpose::CrlSign);

        // Adjust dates for clock skew
        let now = SystemTime::now();
        params.not_before = time::OffsetDateTime::from(now - std::time::Duration::from_secs(86400 * 365));
        params.not_after = time::OffsetDateTime::from(now + std::time::Duration::from_secs(86400 * 365 * 10));

        let key_pair = KeyPair::generate().expect("Failed to generate CA key");
        let cert = params.self_signed(&key_pair).expect("Failed to sign CA cert");

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        fs::write(&cert_path, &cert_pem).expect("Failed to write CA cert");
        fs::write(&key_path, &key_pem).expect("Failed to write CA key");

        CA {
            cert,
            key_pair,
            cert_pem,
        }
    }
}

pub fn delete_ca(ca_dir: PathBuf) {
    let cert_path = ca_dir.join("ca.crt");
    let key_path = ca_dir.join("ca.key");
    let _ = fs::remove_file(cert_path);
    let _ = fs::remove_file(key_path);
}
