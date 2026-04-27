//! cert_manager IO tests using a tempdir + a real self-signed DER cert.

use std::fs;
use std::path::PathBuf;

use opcuasim_core::cert_manager::{
    delete_certificate, list_certificates, move_certificate, CertRole,
};

const TEST_CERT_DER: &[u8] = include_bytes!("fixtures/test_cert.der");

fn setup_pki(tmp: &tempfile::TempDir) -> PathBuf {
    let pki = tmp.path().to_path_buf();
    fs::create_dir_all(pki.join("trusted")).unwrap();
    fs::create_dir_all(pki.join("rejected")).unwrap();
    pki
}

#[test]
fn list_empty_when_dir_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let list =
        list_certificates(tmp.path(), CertRole::Trusted).expect("list ok");
    assert!(list.is_empty());
}

#[test]
fn list_then_move_then_delete() {
    let tmp = tempfile::tempdir().unwrap();
    let pki = setup_pki(&tmp);

    let reject_path = pki.join("rejected").join("foo.der");
    fs::write(&reject_path, TEST_CERT_DER).unwrap();

    let listed = list_certificates(&pki, CertRole::Rejected).expect("list ok");
    assert_eq!(listed.len(), 1);
    let summary = &listed[0];
    assert_eq!(summary.file_name, "foo.der");
    assert_eq!(summary.role, CertRole::Rejected);
    assert!(
        !summary.thumbprint.is_empty(),
        "thumbprint should be set, got summary={summary:?}"
    );
    assert!(
        !summary.subject_cn.is_empty(),
        "subject_cn should be set"
    );
    assert!(
        !summary.valid_from.is_empty(),
        "valid_from should be set"
    );

    let new_path =
        move_certificate(&pki, &reject_path, CertRole::Trusted).expect("move ok");
    assert!(new_path.starts_with(pki.join("trusted")));
    assert!(!reject_path.exists());
    assert!(new_path.exists());

    let trusted = list_certificates(&pki, CertRole::Trusted).expect("list ok");
    assert_eq!(trusted.len(), 1);
    let rejected = list_certificates(&pki, CertRole::Rejected).expect("list ok");
    assert!(rejected.is_empty());

    delete_certificate(&new_path).expect("delete ok");
    assert!(!new_path.exists());
}

#[test]
fn delete_missing_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    delete_certificate(&tmp.path().join("nonexistent.der")).expect("idempotent");
}
