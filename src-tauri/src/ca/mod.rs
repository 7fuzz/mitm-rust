use std::fs;
use std::path::Path;
use rcgen::{
    BasicConstraints, CertificateParams, IsCa, KeyPair, KeyUsagePurpose, SanType,
};

pub struct RootCa {
    pub cert_pem: String,
    pub key_pem: String,
}

impl RootCa {
    pub fn load_or_generate<P: AsRef<Path>>(ca_dir: P) -> Result<Self, String> {
        let dir = ca_dir.as_ref();
        if !dir.exists() {
            fs::create_dir_all(dir).map_err(|e| format!("Failed to create CA dir: {}", e))?;
        }

        let cert_path = dir.join("ca.crt");
        let key_path = dir.join("ca.key");

        if cert_path.exists() && key_path.exists() {
            let cert_pem = fs::read_to_string(&cert_path).map_err(|e| e.to_string())?;
            let key_pem = fs::read_to_string(&key_path).map_err(|e| e.to_string())?;
            
            Ok(Self {
                cert_pem,
                key_pem,
            })
        } else {
            let mut params = CertificateParams::default();
            params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            params.key_usages = vec![
                KeyUsagePurpose::KeyCertSign,
                KeyUsagePurpose::CrlSign,
                KeyUsagePurpose::DigitalSignature,
            ];
            
            let mut dn = rcgen::DistinguishedName::new();
            dn.push(rcgen::DnType::CommonName, "MITM Rust Local Root CA");
            dn.push(rcgen::DnType::OrganizationName, "MITM Rust Dev");
            params.distinguished_name = dn;

            let key_pair = KeyPair::generate().map_err(|e| format!("Failed to generate CA key pair: {}", e))?;
            let cert = params.self_signed(&key_pair).map_err(|e| format!("Failed to self sign CA cert: {}", e))?;
            
            let cert_pem = cert.pem();
            let key_pem = key_pair.serialize_pem();

            fs::write(&cert_path, &cert_pem).map_err(|e| format!("Failed to save ca.crt: {}", e))?;
            fs::write(&key_path, &key_pem).map_err(|e| format!("Failed to save ca.key: {}", e))?;

            Ok(Self {
                cert_pem,
                key_pem,
            })
        }
    }

    pub fn issue_leaf_cert(&self, domain: &str) -> Result<(String, String), String> {
        let ca_key_pair = KeyPair::from_pem(&self.key_pem)
            .map_err(|e| format!("Failed to parse CA key pair: {}", e))?;

        let mut params = CertificateParams::new(vec![domain.to_string()]).map_err(|e| e.to_string())?;
        params.subject_alt_names = vec![SanType::DnsName(domain.to_string().try_into().map_err(|e| format!("{}", e))?)];
        
        let leaf_key_pair = KeyPair::generate().map_err(|e| format!("Failed to generate leaf key pair: {}", e))?;
        
        let ca_cert_params = CertificateParams::from_ca_cert_pem(&self.cert_pem)
            .map_err(|e| format!("Failed to parse CA cert pem: {}", e))?;
        let ca_cert = ca_cert_params.self_signed(&ca_key_pair)
            .map_err(|e| format!("Failed to sign with CA: {}", e))?;

        let leaf_cert = params.signed_by(&leaf_key_pair, &ca_cert, &ca_key_pair)
            .map_err(|e| format!("Failed to sign leaf cert: {}", e))?;

        Ok((leaf_cert.pem(), leaf_key_pair.serialize_pem()))
    }
}
