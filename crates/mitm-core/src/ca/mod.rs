mod api;
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
    /// The certificate as stored; `cert` is re-signed on load and its bytes differ.
    pub cert_der: Vec<u8>,
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
            cert_der: self.cert_der.clone(),
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
        let hash = ring::digest::digest(&ring::digest::SHA256, &self.cert_der);
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
            CaFormat::Der => &self.cert_der,
        };
        fs::write(path, bytes).map_err(|e| format!("Failed to write CA file {}: {}", path.display(), e))
    }
}

fn pem_to_der(pem: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let body: String = pem.lines().filter(|line| !line.starts_with("-----")).map(str::trim).collect();
    base64::engine::general_purpose::STANDARD.decode(body).ok()
}

#[cfg(unix)]
fn set_mode(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = fs::metadata(path) else { return };
    if meta.permissions().mode() & 0o777 != mode {
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
    }
}

#[cfg(not(unix))]
fn set_mode(_path: &std::path::Path, _mode: u32) {}

fn create_private_dir(dir: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    set_mode(dir, 0o700);
    Ok(())
}

fn write_private_file(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)?.write_all(contents.as_bytes())?;
    set_mode(path, 0o600);
    Ok(())
}

pub fn get_default_ca_dir() -> PathBuf {
    let app_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mitm")
        .join("ca");
    let _ = create_private_dir(&app_dir);
    app_dir
}

pub fn get_ca(ca_dir: PathBuf) -> CA {
    let cert_path = ca_dir.join("ca.crt");
    let key_path = ca_dir.join("ca.key");

    if cert_path.exists() && key_path.exists() {
        set_mode(&ca_dir, 0o700);
        set_mode(&key_path, 0o600);
        let cert_pem = fs::read_to_string(&cert_path).expect("Failed to read CA cert");
        let key_pem = fs::read_to_string(&key_path).expect("Failed to read CA key");
        
        let key_pair = KeyPair::from_pem(&key_pem).expect("Failed to parse CA key");
        let params = CertificateParams::from_ca_cert_pem(&cert_pem).expect("Failed to parse CA cert params");
        let cert = params.self_signed(&key_pair).expect("Failed to reconstruct CA cert");

        let cert_der = pem_to_der(&cert_pem).expect("Failed to decode CA cert");

        CA {
            cert,
            key_pair,
            cert_pem,
            cert_der,
        }
    } else {
        create_private_dir(&ca_dir).expect("Failed to create CA directory");

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
        write_private_file(&key_path, &key_pem).expect("Failed to write CA key");

        let cert_der = cert.der().to_vec();

        CA {
            cert,
            key_pair,
            cert_pem,
            cert_der,
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

    #[cfg(unix)]
    #[test]
    fn ca_files_are_private_and_loose_ones_tightened() {
        use std::os::unix::fs::PermissionsExt;
        let mode = |p: &std::path::Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        let dir = std::env::temp_dir().join(format!("mitm_ca_perm_{}", uuid::Uuid::new_v4()));
        get_ca(dir.clone());
        let key = dir.join("ca.key");
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&key), 0o600);

        fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        get_ca(dir.clone());
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&key), 0o600);
        let _ = fs::remove_dir_all(&dir);
    }

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
        assert_eq!(der_content, ca.cert_der);
        assert_eq!(der_content, pem_to_der(&ca.cert_pem).unwrap());

        let reloaded = get_ca(temp_dir.clone());
        assert_eq!(reloaded.cert_der, ca.cert_der);
        assert_eq!(reloaded.info(&temp_dir).unwrap().fingerprint, info.fingerprint);

        delete_ca(temp_dir.clone());
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
