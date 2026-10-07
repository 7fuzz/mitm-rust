use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, Certificate
};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaInfo {
    pub common_name: String,
    pub organization: String,
    pub not_before: String,
    pub not_after: String,
    pub fingerprint: String,
    pub cert_path: String,
    pub ca_dir: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaFormat {
    Pem,
    Der,
}

fn dn_value_to_string(v: &rcgen::DnValue) -> String {
    match v {
        rcgen::DnValue::Utf8String(s) => s.clone(),
        rcgen::DnValue::PrintableString(s) => s.to_string(),
        _ => format!("{:?}", v),
    }
}

pub struct CA {
    pub cert: Certificate,
    pub key_pair: KeyPair,
    pub cert_pem: String,
}

impl CA {
    pub fn clone_shim(&self) -> CA {
        let key_pem = self.key_pair.serialize_pem();
        let key_pair = KeyPair::from_pem(&key_pem).unwrap();
        let params = CertificateParams::from_ca_cert_pem(&self.cert_pem).unwrap();
        let cert = params.self_signed(&key_pair).unwrap();

        CA {
            cert,
            key_pair,
            cert_pem: self.cert_pem.clone(),
        }
    }

    pub fn info(&self, ca_dir: &std::path::Path) -> Result<CaInfo, String> {
        let params = CertificateParams::from_ca_cert_pem(&self.cert_pem)
            .map_err(|e| format!("Failed to parse CA cert: {}", e))?;
        let common_name = params.distinguished_name.get(&DnType::CommonName)
            .map(dn_value_to_string)
            .unwrap_or_default();
        let organization = params.distinguished_name.get(&DnType::OrganizationName)
            .map(dn_value_to_string)
            .unwrap_or_default();
        let not_before = params.not_before
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| params.not_before.to_string());
        let not_after = params.not_after
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| params.not_after.to_string());
        let hash = ring::digest::digest(&ring::digest::SHA256, self.cert.der().as_ref());
        let fingerprint = hash.as_ref().iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(":");

        Ok(CaInfo {
            common_name,
            organization,
            not_before,
            not_after,
            fingerprint,
            cert_path: ca_dir.join("ca.crt").to_string_lossy().to_string(),
            ca_dir: ca_dir.to_string_lossy().to_string(),
        })
    }

    pub fn export(&self, path: &std::path::Path, format: CaFormat) -> Result<(), String> {
        let bytes = match format {
            CaFormat::Pem => self.cert_pem.as_bytes(),
            CaFormat::Der => self.cert.der().as_ref(),
        };
        fs::write(path, bytes).map_err(|e| format!("Failed to write CA file {}: {}", path.display(), e))
    }
}

pub fn get_default_ca_dir() -> PathBuf {
    let app_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mitm")
        .join("ca");
    if !app_dir.exists() {
        let _ = std::fs::create_dir_all(&app_dir);
    }
    app_dir
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ca_lifecycle_and_info() {
        let temp_dir = std::env::temp_dir().join(format!("mitm_ca_test_{}", uuid::Uuid::new_v4()));
        let ca = get_ca(temp_dir.clone());
        let info = ca.info(&temp_dir).unwrap();
        assert_eq!(info.common_name, "MITM Rust CA");
        assert_eq!(info.organization, "MITM Rust");
        assert!(!info.fingerprint.is_empty());
        assert!(info.fingerprint.contains(':'));

        let pem_path = temp_dir.join("export.crt");
        ca.export(&pem_path, CaFormat::Pem).unwrap();
        let pem_content = std::fs::read_to_string(&pem_path).unwrap();
        assert!(pem_content.contains("BEGIN CERTIFICATE"));

        let der_path = temp_dir.join("export.cer");
        ca.export(&der_path, CaFormat::Der).unwrap();
        let der_content = std::fs::read(&der_path).unwrap();
        assert_eq!(der_content, ca.cert.der().as_ref());

        delete_ca(temp_dir.clone());
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
