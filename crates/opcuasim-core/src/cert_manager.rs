//! Trust-list / cert-store file IO for the master's PKI directory.
//!
//! Layout (inherited from async-opcua-crypto):
//! ```text
//! pki/
//!   own/cert.der
//!   private/private.pem
//!   trusted/<thumbprint>.der
//!   rejected/<thumbprint>.der
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use x509_parser::prelude::*;

use crate::error::OpcUaSimError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertRole {
    Trusted,
    Rejected,
}

impl CertRole {
    pub fn dir_name(self) -> &'static str {
        match self {
            CertRole::Trusted => "trusted",
            CertRole::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CertSummary {
    pub path: PathBuf,
    pub file_name: String,
    pub role: CertRole,
    pub thumbprint: String,
    pub subject_cn: String,
    pub issuer_cn: String,
    pub valid_from: String,
    pub valid_to: String,
}

pub fn list_certificates(
    pki_dir: &Path,
    role: CertRole,
) -> Result<Vec<CertSummary>, OpcUaSimError> {
    let dir = pki_dir.join(role.dir_name());
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)
        .map_err(|e| OpcUaSimError::ServerError(format!("read_dir {dir:?}: {e}")))?
    {
        let entry =
            entry.map_err(|e| OpcUaSimError::ServerError(format!("read_dir entry: {e}")))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());
        if !matches!(ext.as_deref(), Some("der") | Some("pem") | Some("crt")) {
            continue;
        }
        out.push(read_summary(&path, role));
    }
    out.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(out)
}

pub fn move_certificate(
    pki_dir: &Path,
    from: &Path,
    to_role: CertRole,
) -> Result<PathBuf, OpcUaSimError> {
    if !from.exists() {
        return Err(OpcUaSimError::ServerError(format!(
            "Source not found: {from:?}"
        )));
    }
    let dest_dir = pki_dir.join(to_role.dir_name());
    fs::create_dir_all(&dest_dir)
        .map_err(|e| OpcUaSimError::ServerError(format!("mkdir {dest_dir:?}: {e}")))?;
    let file_name = from
        .file_name()
        .ok_or_else(|| OpcUaSimError::ServerError("source has no filename".into()))?;
    let dest = dest_dir.join(file_name);
    fs::rename(from, &dest)
        .map_err(|e| OpcUaSimError::ServerError(format!("rename {from:?} -> {dest:?}: {e}")))?;
    Ok(dest)
}

pub fn delete_certificate(path: &Path) -> Result<(), OpcUaSimError> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(path)
        .map_err(|e| OpcUaSimError::ServerError(format!("remove {path:?}: {e}")))
}

fn read_summary(path: &Path, role: CertRole) -> CertSummary {
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("?")
        .to_string();
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(_) => {
            return empty_summary(path.to_path_buf(), file_name, role, "<unreadable>");
        }
    };
    parse_summary(&bytes, path.to_path_buf(), file_name, role)
}

fn empty_summary(
    path: PathBuf,
    file_name: String,
    role: CertRole,
    subject: &str,
) -> CertSummary {
    CertSummary {
        path,
        file_name,
        role,
        thumbprint: String::new(),
        subject_cn: subject.to_string(),
        issuer_cn: String::new(),
        valid_from: String::new(),
        valid_to: String::new(),
    }
}

fn parse_summary(
    bytes: &[u8],
    path: PathBuf,
    file_name: String,
    role: CertRole,
) -> CertSummary {
    let der_bytes: Vec<u8> = match X509Certificate::from_der(bytes) {
        Ok(_) => bytes.to_vec(),
        Err(_) => match ::pem::parse(bytes) {
            Ok(p) => p.into_contents(),
            Err(_) => {
                return empty_summary(path, file_name, role, "<unparseable>");
            }
        },
    };
    let (_, cert) = match X509Certificate::from_der(&der_bytes) {
        Ok(v) => v,
        Err(_) => {
            return empty_summary(path, file_name, role, "<unparseable>");
        }
    };
    let subject_cn = first_cn(cert.subject()).unwrap_or_else(|| cert.subject().to_string());
    let issuer_cn = first_cn(cert.issuer()).unwrap_or_else(|| cert.issuer().to_string());
    let valid_from = cert.validity().not_before.to_string();
    let valid_to = cert.validity().not_after.to_string();
    let thumbprint = sha1_hex(&der_bytes);
    CertSummary {
        path,
        file_name,
        role,
        thumbprint,
        subject_cn,
        issuer_cn,
        valid_from,
        valid_to,
    }
}

fn first_cn(name: &x509_parser::x509::X509Name<'_>) -> Option<String> {
    name.iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .map(|s| s.to_string())
}

fn sha1_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let digest = sha1_smol::Sha1::from(bytes).digest().bytes();
    let mut s = String::with_capacity(40);
    for b in digest {
        let _ = write!(s, "{b:02x}");
    }
    s
}
