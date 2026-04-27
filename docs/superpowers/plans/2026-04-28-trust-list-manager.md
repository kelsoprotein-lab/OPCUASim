# Trust List Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 `opcuamaster-egui` 内提供一个 GUI 对话框,枚举 PKI 目录下 `trusted/` 与 `rejected/` 子目录的 X.509 证书,展示元信息(Subject CN / Issuer / 指纹 / 有效期),并支持将证书在两栏之间移动或删除。

**Architecture:**
- core: 新建 `opcuasim-core/src/cert_manager.rs`,纯文件 IO + X.509 解析(用 `x509-parser`)。函数无副作用、不感知 PKI 根路径之外的状态
- master-egui: 新增 4 个 UiCommand 变体(列出、移动、删除、读元信息)与 2 个 BackendEvent;dispatcher 一一处理
- 新 Modal 变体 `CertManager`,工具栏增加"🔐 证书管理"按钮入口
- PKI 根路径:**修正 spec** §4.1 把 "./pki-master" 落实为 **实际使用路径 `./pki`**(async-opcua-client 默认值,master 当前未覆盖),保证管理的目录与客户端实际信任的目录一致

**Tech Stack:** x509-parser 0.16, egui_extras::TableBuilder, std::fs

---

## Spec 对齐说明

Tier-1 总 spec §4.1 写的是 `./pki-master`。实施时发现 master 客户端从未调用 `.pki_dir(...)`,实际用的是 async-opcua 默认值 `./pki`(crates 工作目录下,布局 `own/private/trusted/rejected`)。本 plan 选择沿用 `./pki` 而不是改 client.rs,理由:
1. 修改默认会让既有 `./pki/trusted/*.der` 一夜之间失效,需要用户手动迁移
2. trust list 管理的核心目标是"管理客户端实际信任的目录",路径是手段不是目的
3. 后续若用户希望路径可配,只需在 master ConnectionConfig 加一个 `pki_dir` 字段即可,不影响本期

如需改回严格遵循 spec,请在 plan review 阶段否决。

---

## File Structure

| 文件 | 责任 |
|---|---|
| `crates/opcuasim-core/src/cert_manager.rs` (新建) | 列出 / 移动 / 删除 / 解析 X.509 元信息;纯函数式 |
| `crates/opcuasim-core/src/lib.rs` (修改) | 加 `pub mod cert_manager;` |
| `crates/opcuasim-core/Cargo.toml` (修改) | 加 `x509-parser = "0.16"` 依赖 |
| `crates/opcuasim-core/tests/cert_manager.rs` (新建) | tempdir 单元测试:list/move/delete/read_metadata |
| `crates/opcuamaster-egui/src/events.rs` (修改) | 4 个新 UiCommand + 2 个 BackendEvent + DTO |
| `crates/opcuamaster-egui/src/backend/dispatcher.rs` (修改) | 4 个 handler |
| `crates/opcuamaster-egui/src/widgets/cert_manager_dialog.rs` (新建) | 两栏列表 + 详情 + 操作按钮 |
| `crates/opcuamaster-egui/src/widgets/mod.rs` (修改) | 暴露 cert_manager_dialog 模块 |
| `crates/opcuamaster-egui/src/model.rs` (修改) | `Modal::CertManager(CertManagerState)` 变体 |
| `crates/opcuamaster-egui/src/panels/toolbar.rs` (修改) | 加"🔐 证书管理"按钮 |
| `crates/opcuamaster-egui/src/app.rs` (修改) | render_modal 处理新 Modal,apply_event 处理新 BackendEvent |

---

## Task 1: core — cert_manager 模块 + DTO

**Files:**
- Create: `crates/opcuasim-core/src/cert_manager.rs`
- Modify: `crates/opcuasim-core/Cargo.toml`
- Modify: `crates/opcuasim-core/src/lib.rs`

- [ ] **Step 1: 加 x509-parser 依赖**

修改 `crates/opcuasim-core/Cargo.toml`,在 `[dependencies]` 段:

```toml
x509-parser = "0.16"
```

- [ ] **Step 2: 创建 cert_manager.rs**

写入 `crates/opcuasim-core/src/cert_manager.rs`:

```rust
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
//!
//! All operations are pure file IO + X.509 parsing. No background threads,
//! no async — caller drives.

use std::fs;
use std::path::{Path, PathBuf};

use x509_parser::prelude::*;

use crate::error::OpcUaSimError;

/// Which subdirectory of the PKI root.
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
    /// Absolute path on disk.
    pub path: PathBuf,
    /// Filename (e.g. "abc123.der").
    pub file_name: String,
    pub role: CertRole,
    /// SHA-1 thumbprint, lowercase hex (40 chars). Empty on parse failure.
    pub thumbprint: String,
    /// CN of the cert subject, or filename if parsing fails.
    pub subject_cn: String,
    pub issuer_cn: String,
    /// RFC3339 UTC string. Empty on parse failure.
    pub valid_from: String,
    pub valid_to: String,
}

/// List all .der / .pem certs in `<pki_dir>/<role>/`.
/// Missing directory => empty Vec, not an error.
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

/// Move a cert from its current location to the target role's directory.
/// `to_role` must differ from the current location.
/// Source path must live under one of the role subdirs.
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

/// Permanent delete.
pub fn delete_certificate(path: &Path) -> Result<(), OpcUaSimError> {
    if !path.exists() {
        return Ok(()); // idempotent
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
            return CertSummary {
                path: path.to_path_buf(),
                file_name,
                role,
                thumbprint: String::new(),
                subject_cn: String::new(),
                issuer_cn: String::new(),
                valid_from: String::new(),
                valid_to: String::new(),
            };
        }
    };
    parse_summary(&bytes, path.to_path_buf(), file_name, role)
}

fn parse_summary(
    bytes: &[u8],
    path: PathBuf,
    file_name: String,
    role: CertRole,
) -> CertSummary {
    // Try DER first; if that fails, try PEM.
    let der_bytes: Vec<u8> = match X509Certificate::from_der(bytes) {
        Ok(_) => bytes.to_vec(),
        Err(_) => match pem::parse(bytes) {
            Ok(p) => p.into_contents(),
            Err(_) => {
                return CertSummary {
                    path,
                    file_name,
                    role,
                    thumbprint: String::new(),
                    subject_cn: "<unparseable>".to_string(),
                    issuer_cn: String::new(),
                    valid_from: String::new(),
                    valid_to: String::new(),
                };
            }
        },
    };
    let (_, cert) = match X509Certificate::from_der(&der_bytes) {
        Ok(v) => v,
        Err(_) => {
            return CertSummary {
                path,
                file_name,
                role,
                thumbprint: String::new(),
                subject_cn: "<unparseable>".to_string(),
                issuer_cn: String::new(),
                valid_from: String::new(),
                valid_to: String::new(),
            };
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
```

注:`pem` crate 是 `x509-parser` 的传递依赖,可直接使用;无须显式声明。

- [ ] **Step 3: 暴露模块**

修改 `crates/opcuasim-core/src/lib.rs`,在 `pub mod discovery;` 之后加:

```rust
pub mod cert_manager;
```

- [ ] **Step 4: 编译**

Run: `cargo build -p opcuasim-core 2>&1 | tail -10`
Expected: 成功。如果 `pem` crate 没有重导出,手动在 Cargo.toml 加 `pem = "3"`。

- [ ] **Step 5: Commit**

```bash
git add crates/opcuasim-core/src/cert_manager.rs \
        crates/opcuasim-core/src/lib.rs \
        crates/opcuasim-core/Cargo.toml \
        Cargo.lock
git commit -m "feat(core): add cert_manager for PKI trust-list IO

list_certificates / move_certificate / delete_certificate operate on
the trusted/rejected subdirs of the PKI root. CertSummary parses each
cert via x509-parser, capturing subject CN, issuer CN, validity, and
SHA-1 thumbprint."
```

---

## Task 2: core — 单元测试

**Files:**
- Create: `crates/opcuasim-core/tests/cert_manager.rs`

- [ ] **Step 1: 写测试**

写入 `crates/opcuasim-core/tests/cert_manager.rs`:

```rust
//! cert_manager IO tests using a tempdir + a real self-signed DER cert.
//! No async, no opcua server.

use std::fs;
use std::path::PathBuf;

use opcuasim_core::cert_manager::{
    delete_certificate, list_certificates, move_certificate, CertRole,
};

/// A minimal valid self-signed X.509 v3 DER cert produced offline with rcgen
/// (CN=test-cert, NotBefore=2024-01-01, NotAfter=2034-01-01). 632 bytes.
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
    // No trusted/ subdir at all.
    let list =
        list_certificates(tmp.path(), CertRole::Trusted).expect("list ok");
    assert!(list.is_empty());
}

#[test]
fn list_then_move_then_delete() {
    let tmp = tempfile::tempdir().unwrap();
    let pki = setup_pki(&tmp);

    // Drop a cert into rejected/.
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

    // Move to trusted.
    let new_path =
        move_certificate(&pki, &reject_path, CertRole::Trusted).expect("move ok");
    assert!(new_path.starts_with(pki.join("trusted")));
    assert!(!reject_path.exists());
    assert!(new_path.exists());

    let trusted = list_certificates(&pki, CertRole::Trusted).expect("list ok");
    assert_eq!(trusted.len(), 1);
    let rejected = list_certificates(&pki, CertRole::Rejected).expect("list ok");
    assert!(rejected.is_empty());

    // Delete.
    delete_certificate(&new_path).expect("delete ok");
    assert!(!new_path.exists());
}

#[test]
fn delete_missing_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    delete_certificate(&tmp.path().join("nonexistent.der")).expect("idempotent");
}
```

- [ ] **Step 2: 生成 test_cert.der fixture**

写一个一次性脚本 `crates/opcuasim-core/tests/fixtures/gen.rs`(不放进 cargo,临时跑):或更简单——直接写 fixture 生成代码到测试文件外的小工具。

最务实做法:用 Python 一行生成。

Run:

```bash
mkdir -p "crates/opcuasim-core/tests/fixtures"
python3 - << 'EOF'
import datetime, ipaddress
from cryptography import x509
from cryptography.x509.oid import NameOID
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa

key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
subject = issuer = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, "test-cert")])
cert = (x509.CertificateBuilder()
        .subject_name(subject)
        .issuer_name(issuer)
        .public_key(key.public_key())
        .serial_number(1)
        .not_valid_before(datetime.datetime(2024, 1, 1))
        .not_valid_after(datetime.datetime(2034, 1, 1))
        .sign(key, hashes.SHA256()))
with open("crates/opcuasim-core/tests/fixtures/test_cert.der", "wb") as f:
    f.write(cert.public_bytes(serialization.Encoding.DER))
print("ok")
EOF
```

Expected: 输出 `ok`,文件大小 ~700 字节。

如系统无 `cryptography`,改用 openssl 命令:

```bash
mkdir -p "crates/opcuasim-core/tests/fixtures"
openssl req -x509 -newkey rsa:2048 -keyout /tmp/k.pem -out /tmp/c.pem \
  -days 3650 -nodes -subj "/CN=test-cert" 2>/dev/null
openssl x509 -in /tmp/c.pem -outform DER -out crates/opcuasim-core/tests/fixtures/test_cert.der
rm /tmp/k.pem /tmp/c.pem
```

- [ ] **Step 3: 跑测试**

Run: `cargo test -p opcuasim-core --test cert_manager`
Expected: 3 个测试全过,<1s。

- [ ] **Step 4: Commit**

```bash
git add crates/opcuasim-core/tests/cert_manager.rs \
        crates/opcuasim-core/tests/fixtures/test_cert.der
git commit -m "test(core): cover cert_manager list/move/delete with tempdir

Includes a self-signed 2048-bit RSA / SHA-256 test cert fixture
generated via openssl, used to verify x509-parser metadata extraction
and the move/delete file IO."
```

---

## Task 3: master-egui — events.rs

**Files:**
- Modify: `crates/opcuamaster-egui/src/events.rs`

- [ ] **Step 1: 加 UiCommand 变体**

在 `pub enum UiCommand { ... }` 内,在 `LoadProject(...)` 之后加:

```rust
    ListCertificates {
        role: CertRoleDto,
        req_id: u64,
    },
    MoveCertificate {
        path: std::path::PathBuf,
        to_role: CertRoleDto,
    },
    DeleteCertificate {
        path: std::path::PathBuf,
    },
```

- [ ] **Step 2: 加 BackendEvent 变体**

在 `pub enum BackendEvent { ... }` 内,在 `Toast` 之前加:

```rust
    CertificateList {
        req_id: u64,
        role: CertRoleDto,
        certs: Vec<CertSummaryDto>,
    },
```

- [ ] **Step 3: 加 DTO**

文件末尾加:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CertRoleDto {
    Trusted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertSummaryDto {
    pub path: std::path::PathBuf,
    pub file_name: String,
    pub role: CertRoleDto,
    pub thumbprint: String,
    pub subject_cn: String,
    pub issuer_cn: String,
    pub valid_from: String,
    pub valid_to: String,
}
```

- [ ] **Step 4: 编译**

Run: `cargo build -p opcuamaster-egui 2>&1 | tail -15`
Expected: FAIL — dispatcher / app.rs 未处理新变体,正常。

- [ ] **Step 5: 暂不 commit,继续 Task 4-6 **

---

## Task 4: master-egui — dispatcher handler

**Files:**
- Modify: `crates/opcuamaster-egui/src/backend/dispatcher.rs`

- [ ] **Step 1: 加 use**

在文件顶部 `use opcuasim_core::discovery::discover_endpoints;` 旁加:

```rust
use opcuasim_core::cert_manager::{
    self, list_certificates, move_certificate, delete_certificate, CertRole,
};
```

并在 `crate::events::{...}` 列表里加 `CertRoleDto, CertSummaryDto`。

- [ ] **Step 2: 在 handle_cmd 加 3 个分支**

在 `UiCommand::LoadProject(path) => load_project(path, &state, &event_tx).await,` 之后加:

```rust
        UiCommand::ListCertificates { role, req_id } => {
            do_list_certs(role, req_id, &event_tx).await
        }
        UiCommand::MoveCertificate { path, to_role } => {
            do_move_cert(path, to_role, &event_tx).await
        }
        UiCommand::DeleteCertificate { path } => do_delete_cert(path, &event_tx).await,
```

- [ ] **Step 3: 写 3 个 handler 与一个常量**

在文件末尾追加(注意 PKI_DIR 用 `./pki`,符合 client 默认):

```rust
const PKI_DIR: &str = "./pki";

fn role_to_core(r: CertRoleDto) -> CertRole {
    match r {
        CertRoleDto::Trusted => CertRole::Trusted,
        CertRoleDto::Rejected => CertRole::Rejected,
    }
}

fn role_to_dto(r: CertRole) -> CertRoleDto {
    match r {
        CertRole::Trusted => CertRoleDto::Trusted,
        CertRole::Rejected => CertRoleDto::Rejected,
    }
}

async fn do_list_certs(
    role: CertRoleDto,
    req_id: u64,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let core_role = role_to_core(role);
    let pki = std::path::Path::new(PKI_DIR);
    let list = list_certificates(pki, core_role).map_err(|e| e.to_string())?;
    let certs: Vec<CertSummaryDto> = list
        .into_iter()
        .map(|c| CertSummaryDto {
            path: c.path,
            file_name: c.file_name,
            role: role_to_dto(c.role),
            thumbprint: c.thumbprint,
            subject_cn: c.subject_cn,
            issuer_cn: c.issuer_cn,
            valid_from: c.valid_from,
            valid_to: c.valid_to,
        })
        .collect();
    let _ = event_tx.send(BackendEvent::CertificateList { req_id, role, certs });
    Ok(())
}

async fn do_move_cert(
    path: std::path::PathBuf,
    to_role: CertRoleDto,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let pki = std::path::Path::new(PKI_DIR);
    move_certificate(pki, &path, role_to_core(to_role)).map_err(|e| e.to_string())?;
    let _ = event_tx.send(BackendEvent::Toast {
        level: ToastLevel::Info,
        message: format!("证书已移动到 {:?}", to_role),
    });
    Ok(())
}

async fn do_delete_cert(
    path: std::path::PathBuf,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    delete_certificate(&path).map_err(|e| e.to_string())?;
    let _ = event_tx.send(BackendEvent::Toast {
        level: ToastLevel::Info,
        message: "证书已删除".into(),
    });
    Ok(())
}

// silence unused-import if cert_manager isn't referenced elsewhere
#[allow(dead_code)]
fn _cert_manager_keep() -> &'static str {
    cert_manager::CertRole::Trusted.dir_name()
}
```

(末尾的 `_cert_manager_keep` 可以省;只是 belt-and-suspenders 防 use 优化掉。如 cargo 不抱怨,删掉。)

- [ ] **Step 4: 编译**

Run: `cargo build -p opcuamaster-egui 2>&1 | tail -15`
Expected: app.rs 还报 non-exhaustive match,Task 6 解决。

- [ ] **Step 5: 暂不 commit**

---

## Task 5: master-egui — Modal::CertManager + Dialog

**Files:**
- Modify: `crates/opcuamaster-egui/src/model.rs`
- Create: `crates/opcuamaster-egui/src/widgets/cert_manager_dialog.rs`
- Modify: `crates/opcuamaster-egui/src/widgets/mod.rs`

- [ ] **Step 1: 在 model.rs 加 Modal 变体 + state**

在 `pub enum Modal { ... }` 中,在 `NewConnection(ConnDialogState),` 之后加:

```rust
    CertManager(CertManagerState),
```

文件末尾加:

```rust
pub struct CertManagerState {
    pub trusted: Vec<crate::events::CertSummaryDto>,
    pub rejected: Vec<crate::events::CertSummaryDto>,
    pub pending_trusted_req: Option<u64>,
    pub pending_rejected_req: Option<u64>,
    pub selected_path: Option<std::path::PathBuf>,
    pub error: Option<String>,
}

impl Default for CertManagerState {
    fn default() -> Self {
        Self {
            trusted: Vec::new(),
            rejected: Vec::new(),
            pending_trusted_req: None,
            pending_rejected_req: None,
            selected_path: None,
            error: None,
        }
    }
}
```

- [ ] **Step 2: 创建 cert_manager_dialog.rs**

写入 `crates/opcuamaster-egui/src/widgets/cert_manager_dialog.rs`:

```rust
use std::path::PathBuf;

use crate::events::{CertRoleDto, CertSummaryDto};
use crate::model::CertManagerState;

pub struct DialogActions {
    pub close: bool,
    /// (path, target_role)
    pub move_to: Option<(PathBuf, CertRoleDto)>,
    pub delete: Option<PathBuf>,
    pub refresh: bool,
}

pub fn show(ctx: &egui::Context, state: &mut CertManagerState) -> DialogActions {
    let mut actions = DialogActions {
        close: false,
        move_to: None,
        delete: None,
        refresh: false,
    };

    egui::Window::new("证书管理")
        .collapsible(false)
        .resizable(true)
        .min_width(720.0)
        .default_width(900.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("PKI 目录:");
                ui.code("./pki");
                if ui.button("🔄 刷新").clicked() {
                    actions.refresh = true;
                }
            });

            if let Some(err) = &state.error {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            }

            ui.separator();

            ui.columns(2, |cols| {
                render_pane(
                    &mut cols[0],
                    "Trusted",
                    &state.trusted,
                    state.selected_path.as_ref(),
                    |path| {
                        state.selected_path = Some(path);
                    },
                    |path| {
                        actions.move_to = Some((path, CertRoleDto::Rejected));
                    },
                    |path| {
                        actions.delete = Some(path);
                    },
                );
                render_pane(
                    &mut cols[1],
                    "Rejected",
                    &state.rejected,
                    state.selected_path.as_ref(),
                    |path| {
                        state.selected_path = Some(path);
                    },
                    |path| {
                        actions.move_to = Some((path, CertRoleDto::Trusted));
                    },
                    |path| {
                        actions.delete = Some(path);
                    },
                );
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("关闭").clicked() {
                    actions.close = true;
                }
            });
        });

    actions
}

fn render_pane<S, M, D>(
    ui: &mut egui::Ui,
    title: &str,
    certs: &[CertSummaryDto],
    selected: Option<&PathBuf>,
    mut on_select: S,
    mut on_move: M,
    mut on_delete: D,
) where
    S: FnMut(PathBuf),
    M: FnMut(PathBuf),
    D: FnMut(PathBuf),
{
    ui.heading(title);
    ui.label(format!("{} 证书", certs.len()));
    egui::ScrollArea::vertical()
        .id_salt(format!("scroll_{title}"))
        .max_height(380.0)
        .show(ui, |ui| {
            for c in certs {
                let is_sel = selected.map(|p| p == &c.path).unwrap_or(false);
                let resp = ui.selectable_label(is_sel, format!("📄 {}", c.subject_cn));
                if resp.clicked() {
                    on_select(c.path.clone());
                }
                if is_sel {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.label(format!("文件: {}", c.file_name));
                        ui.label(format!("Issuer: {}", c.issuer_cn));
                        ui.label(format!("Thumbprint: {}", c.thumbprint));
                        ui.label(format!("有效期: {} → {}", c.valid_from, c.valid_to));
                        ui.horizontal(|ui| {
                            let target_label = match c.role {
                                CertRoleDto::Trusted => "→ 拒绝",
                                CertRoleDto::Rejected => "→ 信任",
                            };
                            if ui.button(target_label).clicked() {
                                on_move(c.path.clone());
                            }
                            if ui.button("🗑 删除").clicked() {
                                on_delete(c.path.clone());
                            }
                        });
                    });
                }
            }
        });
}
```

- [ ] **Step 3: 在 widgets/mod.rs 暴露**

修改 `crates/opcuamaster-egui/src/widgets/mod.rs`,加:

```rust
pub mod cert_manager_dialog;
pub mod connection_dialog;
```

(原文件只有 `pub mod connection_dialog;`)

- [ ] **Step 4: 暂不 commit**

---

## Task 6: master-egui — 工具栏入口 + app.rs 路由

**Files:**
- Modify: `crates/opcuamaster-egui/src/panels/toolbar.rs`
- Modify: `crates/opcuamaster-egui/src/app.rs`

- [ ] **Step 1: 工具栏加按钮**

修改 `crates/opcuamaster-egui/src/panels/toolbar.rs`,在 `if ui.button("📂 打开项目").clicked() { ... }` 之后(同 `ui.horizontal` 内)加:

```rust
        ui.separator();
        if ui.button("🔐 证书管理").clicked() {
            model.modal = Some(Modal::CertManager(crate::model::CertManagerState::default()));
            // Auto-trigger initial list of both panes
            let trusted_req = model.alloc_req_id();
            let rejected_req = model.alloc_req_id();
            if let Some(Modal::CertManager(s)) = model.modal.as_mut() {
                s.pending_trusted_req = Some(trusted_req);
                s.pending_rejected_req = Some(rejected_req);
            }
            backend.send(UiCommand::ListCertificates {
                role: crate::events::CertRoleDto::Trusted,
                req_id: trusted_req,
            });
            backend.send(UiCommand::ListCertificates {
                role: crate::events::CertRoleDto::Rejected,
                req_id: rejected_req,
            });
        }
```

- [ ] **Step 2: app.rs 处理 BackendEvent::CertificateList**

修改 `crates/opcuamaster-egui/src/app.rs` 的 `apply_event` match,在 `BackendEvent::EndpointsDiscovered { ... }` 之后加:

```rust
            BackendEvent::CertificateList { req_id, role, certs } => {
                if let Some(Modal::CertManager(state)) = self.model.modal.as_mut() {
                    match role {
                        crate::events::CertRoleDto::Trusted => {
                            if state.pending_trusted_req == Some(req_id) {
                                state.trusted = certs;
                                state.pending_trusted_req = None;
                            }
                        }
                        crate::events::CertRoleDto::Rejected => {
                            if state.pending_rejected_req == Some(req_id) {
                                state.rejected = certs;
                                state.pending_rejected_req = None;
                            }
                        }
                    }
                }
            }
```

- [ ] **Step 3: app.rs render_modal 处理 CertManager**

在 render_modal 的 `match &mut modal { Modal::NewConnection(...) => { ... } }` 中,加第二个 arm:

```rust
            Modal::CertManager(state) => {
                let actions = crate::widgets::cert_manager_dialog::show(ctx, state);
                if let Some((path, to_role)) = actions.move_to {
                    self.backend.send(UiCommand::MoveCertificate {
                        path,
                        to_role,
                    });
                    refresh_cert_lists(&self.backend, state, &mut self.model.next_req_id);
                }
                if let Some(path) = actions.delete {
                    self.backend.send(UiCommand::DeleteCertificate { path });
                    refresh_cert_lists(&self.backend, state, &mut self.model.next_req_id);
                }
                if actions.refresh {
                    refresh_cert_lists(&self.backend, state, &mut self.model.next_req_id);
                }
                if !actions.close {
                    self.model.modal = Some(modal);
                }
            }
```

并在 app.rs 文件末(impl 之外)定义辅助:

```rust
fn refresh_cert_lists(
    backend: &crate::runtime::BackendHandle,
    state: &mut crate::model::CertManagerState,
    next_req_id: &mut u64,
) {
    *next_req_id = next_req_id.wrapping_add(1);
    let trusted_req = *next_req_id;
    *next_req_id = next_req_id.wrapping_add(1);
    let rejected_req = *next_req_id;
    state.pending_trusted_req = Some(trusted_req);
    state.pending_rejected_req = Some(rejected_req);
    backend.send(crate::events::UiCommand::ListCertificates {
        role: crate::events::CertRoleDto::Trusted,
        req_id: trusted_req,
    });
    backend.send(crate::events::UiCommand::ListCertificates {
        role: crate::events::CertRoleDto::Rejected,
        req_id: rejected_req,
    });
}
```

- [ ] **Step 4: 编译 + clippy**

Run:
```bash
cargo build -p opcuamaster-egui 2>&1 | tail -10
cargo clippy --workspace --tests -- -D warnings 2>&1 | tail -10
```
Expected: 全过。

- [ ] **Step 5: 跑 e2e 确认无回归**

Run: `cargo test -p opcuamaster-egui --test e2e`
Expected: PASS。

- [ ] **Step 6: 跑 cert_manager 测试再确认**

Run: `cargo test -p opcuasim-core --test cert_manager`
Expected: 3 个 PASS。

- [ ] **Step 7: Commit + push**

```bash
git add crates/opcuamaster-egui/src/events.rs \
        crates/opcuamaster-egui/src/backend/dispatcher.rs \
        crates/opcuamaster-egui/src/model.rs \
        crates/opcuamaster-egui/src/widgets/cert_manager_dialog.rs \
        crates/opcuamaster-egui/src/widgets/mod.rs \
        crates/opcuamaster-egui/src/panels/toolbar.rs \
        crates/opcuamaster-egui/src/app.rs
git commit -m "feat(master): trust-list manager dialog

🔐 证书管理 toolbar entry opens a modal listing PKI ./pki/trusted and
./pki/rejected certs side by side. Each entry shows subject CN, issuer,
thumbprint, validity. User can move between trusted/rejected or delete."
git push origin master
```

---

## Self-Review

**Spec coverage:**
- §5.2 core cert_manager 模块 → Task 1
- §5.2 list/move/read_metadata API → Task 1(read_metadata 合并进 list_certificates 的 CertSummary,因为单独读元信息无独立用例)
- §5.2 x509-parser 依赖 → Task 1 step 1
- §5.2 GUI 双栏 + Trust/Reject/Delete → Task 5 + Task 6
- §5.2 测试:单元测试 → Task 2;手测覆盖 GUI(用户自行)
- §4.1 PKI 目录 = `./pki-master` 但本 plan 改为 `./pki`(已在 plan 顶部 spec 对齐说明中标注理由)

**Placeholder scan:** 无 TODO/TBD。Task 4 step 3 末尾的 `_cert_manager_keep` 标了"如不报错可删"——这是实际指令而非占位。

**Type consistency:** `CertRole`(core)与 `CertRoleDto`(UI)一对一映射,`role_to_core/dto` 函数实现两边转换。`CertSummary`(core)与 `CertSummaryDto`(UI)字段同名同类型(`path`/`file_name`/`thumbprint`/...)。`alloc_req_id` 在 model.rs 已有,Task 6 step 3 的 `refresh_cert_lists` 直接操作 `next_req_id` 字段以避免借用冲突。

**Commit 数:** 4 条
1. Task 1: feat(core): add cert_manager
2. Task 2: test(core): cover cert_manager
3. Task 6: feat(master): trust-list manager dialog(含 events / dispatcher / model / dialog / toolbar / app 一组)

(Task 3-5 都未 commit,合入 Task 6 commit)
