# OPC UA 采集主站 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an OPC UA master station (client) desktop app that connects to OPC UA servers, browses nodes, reads data via subscription and polling, with a pluggable output system.

**Architecture:** Tauri 2 desktop app with Rust backend (async-opcua-client + tokio) and Vue 3 + TypeScript frontend. Core library (`opcuasim-core`) handles all OPC UA logic; Tauri app layer bridges to frontend via IPC commands and events. Frontend follows the same grid layout and Catppuccin Mocha dark theme as the existing ModbusSim and IEC104 Simulator projects.

**Tech Stack:** Rust, Tokio, async-opcua-client, Tauri 2, Vue 3, TypeScript, Vite 8, @tanstack/vue-virtual

**Reference projects:** ModbusSim (`/Users/daichangyu/Library/Mobile Documents/com~apple~CloudDocs/code/ModbusSim`) and IEC104 Simulator (`/Users/daichangyu/Library/Mobile Documents/com~apple~CloudDocs/code/IEC60870-5-104-Simulator`). Follow their patterns exactly.

---

## File Structure

### Rust Backend

```
crates/
├── opcuasim-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Module declarations
│       ├── error.rs            # OpcUaSimError enum with category()
│       ├── node.rs             # MonitoredNode, OpcDataType, AccessMode, NodeGroup
│       ├── config.rs           # ConnectionConfig, AuthConfig, ProjectFile serialization
│       ├── output.rs           # DataOutput trait + LogOutput implementation
│       ├── log_entry.rs        # LogEntry, Direction, ServiceType
│       ├── log_collector.rs    # Thread-safe ring buffer log collector
│       ├── reconnect.rs        # ReconnectPolicy with exponential backoff
│       ├── client.rs           # OpcUaConnection: connect/disconnect/reconnect, session management
│       ├── browse.rs           # browse_root, browse_node, read_node_attributes
│       ├── subscription.rs     # Subscription manager: create/modify/delete subscriptions
│       └── polling.rs          # Polling manager: tokio interval-based reads
└── opcuamaster-app/
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    └── src/
        ├── main.rs
        ├── lib.rs              # Tauri app setup, command registration
        ├── state.rs            # AppState, ConnectionState, DTOs
        └── commands.rs         # All Tauri IPC command handlers
```

### Vue 3 Frontend

```
master-frontend/
├── package.json
├── index.html
├── vite.config.ts
├── tsconfig.json
├── tsconfig.app.json
├── tsconfig.node.json
└── src/
    ├── main.ts
    ├── App.vue                 # Grid layout root component
    ├── types.ts                # TypeScript interfaces matching Rust DTOs
    ├── composables/
    │   └── useDialog.ts        # Re-export from shared-frontend
    └── components/
        ├── Toolbar.vue         # Connection CRUD, project file, browse trigger
        ├── ConnectionTree.vue  # Address space view + group view
        ├── DataTable.vue       # Virtual-scrolled monitored data table
        ├── ValuePanel.vue      # Selected node attributes + read/write
        ├── BrowsePanel.vue     # Modal node browser with lazy-load tree
        └── LogPanel.vue        # Collapsible communication log
```

### Shared Frontend (reuse from ModbusSim pattern)

```
shared-frontend/
├── package.json
├── tsconfig.json
└── src/
    ├── index.ts
    ├── types/
    │   └── common.ts           # LogEntry, DialogMode, DialogState
    ├── composables/
    │   ├── useDialog.ts
    │   ├── useLogPanel.ts
    │   ├── useLogFilter.ts
    │   └── useErrorHandler.ts
    └── components/
        └── AppDialog.vue
```

---

## Task 1: Project Scaffolding — Cargo Workspace + npm Workspace

**Files:**
- Create: `Cargo.toml`
- Create: `package.json`
- Create: `crates/opcuasim-core/Cargo.toml`
- Create: `crates/opcuasim-core/src/lib.rs`
- Create: `crates/opcuamaster-app/Cargo.toml`
- Create: `crates/opcuamaster-app/build.rs`
- Create: `crates/opcuamaster-app/src/main.rs`
- Create: `crates/opcuamaster-app/src/lib.rs`
- Create: `crates/opcuamaster-app/tauri.conf.json`

- [ ] **Step 1: Create root Cargo.toml**

```toml
[workspace]
members = ["crates/opcuasim-core", "crates/opcuamaster-app"]
resolver = "2"
```

- [ ] **Step 2: Create root package.json**

```json
{
  "private": true,
  "workspaces": [
    "shared-frontend",
    "master-frontend"
  ]
}
```

- [ ] **Step 3: Create opcuasim-core/Cargo.toml**

```toml
[package]
name = "opcuasim-core"
version = "0.1.0"
edition = "2021"

[dependencies]
async-opcua-client = "0.18"
async-opcua-types = "0.18"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2.0"
uuid = { version = "1", features = ["v4", "serde"] }
log = "0.4"

[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
tempfile = "3"
```

- [ ] **Step 4: Create opcuasim-core/src/lib.rs**

```rust
pub mod error;
pub mod node;       // MonitoredNode, NodeGroup, BrowseResultItem, NodeAttributes
pub mod config;     // ConnectionConfig, AuthConfig, ProjectFile (includes security config)
pub mod output;
pub mod log_entry;
pub mod log_collector;
pub mod reconnect;
pub mod client;
pub mod browse;
pub mod subscription;
pub mod polling;
```

Note: The spec listed `group.rs` and `security.rs` as separate files, but they are merged into `node.rs` (NodeGroup) and `config.rs` (AuthConfig, security policy/mode) respectively for simplicity.

- [ ] **Step 5: Create opcuamaster-app/Cargo.toml**

```toml
[package]
name = "opcuamaster-app"
version = "0.1.0"
description = "OPCUAMaster - Cross-platform OPC UA Master Station"
edition = "2021"
rust-version = "1.77.2"

[lib]
name = "opcuamaster_app_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2.5.6", features = [] }

[dependencies]
opcuasim-core = { path = "../opcuasim-core" }
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
log = "0.4"
tauri = { version = "2.10.3", features = [] }
tauri-plugin-log = "2"
tauri-plugin-dialog = "2"
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 6: Create opcuamaster-app/build.rs**

```rust
fn main() {
  tauri_build::build()
}
```

- [ ] **Step 7: Create opcuamaster-app/src/main.rs**

```rust
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    opcuamaster_app_lib::run();
}
```

- [ ] **Step 8: Create opcuamaster-app/src/lib.rs (minimal)**

```rust
mod commands;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::list_connections,
        ])
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 9: Create opcuamaster-app/tauri.conf.json**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "OPCUAMaster",
  "version": "0.1.0",
  "identifier": "com.opcuamaster.dev",
  "build": {
    "devUrl": "http://localhost:5178",
    "beforeDevCommand": {
      "script": "npm run dev",
      "cwd": "../../master-frontend"
    },
    "frontendDist": "../../master-frontend/dist"
  },
  "app": {
    "windows": [
      {
        "title": "OPCUAMaster - OPC UA Master Station",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "macOS": {
      "minimumSystemVersion": "10.15"
    },
    "windows": {
      "webviewInstallMode": {
        "type": "downloadBootstrapper"
      }
    }
  }
}
```

- [ ] **Step 10: Create placeholder state.rs and commands.rs**

`crates/opcuamaster-app/src/state.rs`:
```rust
use std::collections::HashMap;
use std::sync::RwLock;

pub struct AppState {
    pub connections: RwLock<HashMap<String, ()>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }
}
```

`crates/opcuamaster-app/src/commands.rs`:
```rust
use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub fn list_connections(_state: State<'_, AppState>) -> Vec<String> {
    vec![]
}
```

- [ ] **Step 11: Verify Rust workspace compiles**

Run: `cargo check --workspace`
Expected: Compiles successfully (may download dependencies)

- [ ] **Step 12: Commit**

```bash
git init
git add Cargo.toml Cargo.lock package.json crates/
git commit -m "feat: scaffold Cargo workspace with opcuasim-core and opcuamaster-app"
```

---

## Task 2: Frontend Scaffolding — shared-frontend + master-frontend

**Files:**
- Create: `shared-frontend/package.json`
- Create: `shared-frontend/tsconfig.json`
- Create: `shared-frontend/src/index.ts`
- Create: `shared-frontend/src/types/common.ts`
- Create: `shared-frontend/src/composables/useDialog.ts`
- Create: `shared-frontend/src/composables/useLogPanel.ts`
- Create: `shared-frontend/src/composables/useLogFilter.ts`
- Create: `shared-frontend/src/composables/useErrorHandler.ts`
- Create: `shared-frontend/src/components/AppDialog.vue`
- Create: `master-frontend/package.json`
- Create: `master-frontend/index.html`
- Create: `master-frontend/vite.config.ts`
- Create: `master-frontend/tsconfig.json`
- Create: `master-frontend/tsconfig.app.json`
- Create: `master-frontend/tsconfig.node.json`
- Create: `master-frontend/src/main.ts`
- Create: `master-frontend/src/App.vue`
- Create: `master-frontend/src/types.ts`
- Create: `master-frontend/src/composables/useDialog.ts`

- [ ] **Step 1: Create shared-frontend/package.json**

```json
{
  "name": "shared-frontend",
  "private": true,
  "version": "0.0.0",
  "type": "module",
  "main": "src/index.ts",
  "dependencies": {
    "vue": "^3.5.30",
    "@tauri-apps/api": "^2.10.1"
  },
  "devDependencies": {
    "@vue/tsconfig": "^0.9.0"
  }
}
```

- [ ] **Step 2: Create shared-frontend/tsconfig.json**

```json
{
  "extends": "@vue/tsconfig/tsconfig.dom.json",
  "compilerOptions": {
    "composite": true,
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.tsbuildinfo",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "erasableSyntaxOnly": true
  },
  "include": ["src/**/*.ts", "src/**/*.vue"]
}
```

- [ ] **Step 3: Create shared-frontend/src/types/common.ts**

```typescript
export interface LogEntry {
  timestamp: string
  direction: string
  service: string
  detail: string
  status?: string
}

export type DialogMode = 'alert' | 'confirm' | 'prompt'

export interface DialogState {
  visible: boolean
  mode: DialogMode
  title: string
  message: string
  inputValue: string
  resolve: ((value: boolean | string | null) => void) | null
}
```

- [ ] **Step 4: Create shared-frontend/src/composables/useDialog.ts**

```typescript
import { reactive, type InjectionKey } from 'vue'
import type { DialogMode, DialogState } from '../types/common'

const state = reactive<DialogState>({
  visible: false,
  mode: 'alert',
  title: '',
  message: '',
  inputValue: '',
  resolve: null,
})

export const dialogKey: InjectionKey<{
  showAlert: (title: string, message?: string) => Promise<void>
  showConfirm: (title: string, message?: string) => Promise<boolean>
  showPrompt: (title: string, message?: string, defaultValue?: string) => Promise<string | null>
}> = Symbol('dialog')

function show(mode: DialogMode, title: string, message = '', defaultValue = ''): Promise<any> {
  return new Promise((resolve) => {
    state.mode = mode
    state.title = title
    state.message = message
    state.inputValue = defaultValue
    state.resolve = resolve
    state.visible = true
  })
}

export function showAlert(title: string, message = ''): Promise<void> {
  return show('alert', title, message).then(() => {})
}

export function showConfirm(title: string, message = ''): Promise<boolean> {
  return show('confirm', title, message)
}

export function showPrompt(title: string, message = '', defaultValue = ''): Promise<string | null> {
  return show('prompt', title, message, defaultValue)
}

export function dialogConfirm() {
  if (state.resolve) {
    if (state.mode === 'alert') state.resolve(true)
    else if (state.mode === 'confirm') state.resolve(true)
    else state.resolve(state.inputValue)
  }
  state.visible = false
}

export function dialogCancel() {
  if (state.resolve) {
    if (state.mode === 'confirm') state.resolve(false)
    else if (state.mode === 'prompt') state.resolve(null)
  }
  state.visible = false
}

export function useDialogState() {
  return state
}
```

- [ ] **Step 5: Create shared-frontend/src/composables/useLogPanel.ts**

```typescript
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { LogEntry } from '../types/common'

export function useLogPanel() {
  const logs = ref<LogEntry[]>([])
  const loading = ref(false)

  async function loadLogs(connectionId: string | null) {
    if (!connectionId) {
      logs.value = []
      return
    }
    loading.value = true
    try {
      logs.value = await invoke<LogEntry[]>('get_communication_logs', {
        connId: connectionId,
        sinceSeq: 0,
      })
    } catch (e) {
      console.error('Failed to load logs:', e)
    } finally {
      loading.value = false
    }
  }

  async function clearLogs(connectionId: string | null) {
    if (!connectionId) return
    try {
      await invoke('clear_communication_logs', { connId: connectionId })
      logs.value = []
    } catch (e) {
      console.error('Failed to clear logs:', e)
    }
  }

  async function exportLogsCsv(connectionId: string | null) {
    if (!connectionId) return
    try {
      const csv = await invoke<string>('export_logs_csv', { connId: connectionId })
      const blob = new Blob([csv], { type: 'text/csv' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `opcua-logs-${connectionId}.csv`
      a.click()
      URL.revokeObjectURL(url)
    } catch (e) {
      console.error('Failed to export logs:', e)
    }
  }

  return { logs, loading, loadLogs, clearLogs, exportLogsCsv }
}
```

- [ ] **Step 6: Create shared-frontend/src/composables/useLogFilter.ts**

```typescript
import { ref, computed, type Ref } from 'vue'
import type { LogEntry } from '../types/common'

export type DirectionFilter = 'all' | 'request' | 'response'

export function useLogFilter(logs: Ref<LogEntry[]>) {
  const directionFilter = ref<DirectionFilter>('all')
  const serviceFilter = ref<string>('all')
  const searchQuery = ref('')

  const availableServices = computed(() => {
    const services = new Set<string>()
    logs.value.forEach((log) => {
      if (log.service) services.add(log.service)
    })
    return Array.from(services).sort()
  })

  const filteredLogs = computed(() => {
    return logs.value.filter((log) => {
      if (directionFilter.value !== 'all' && log.direction.toLowerCase() !== directionFilter.value) {
        return false
      }
      if (serviceFilter.value !== 'all' && log.service !== serviceFilter.value) {
        return false
      }
      if (searchQuery.value) {
        const q = searchQuery.value.toLowerCase()
        return (
          log.detail.toLowerCase().includes(q) ||
          log.service.toLowerCase().includes(q) ||
          (log.status && log.status.toLowerCase().includes(q))
        )
      }
      return true
    })
  })

  const filterSummary = computed(() => {
    const total = logs.value.length
    const shown = filteredLogs.value.length
    return total === shown ? `${total} entries` : `${shown} / ${total} entries`
  })

  function resetFilters() {
    directionFilter.value = 'all'
    serviceFilter.value = 'all'
    searchQuery.value = ''
  }

  return {
    directionFilter,
    serviceFilter,
    searchQuery,
    availableServices,
    filteredLogs,
    filterSummary,
    resetFilters,
  }
}
```

- [ ] **Step 7: Create shared-frontend/src/composables/useErrorHandler.ts**

```typescript
import { ref } from 'vue'

export interface Toast {
  id: number
  message: string
  level: 'error' | 'warning' | 'info'
  persistent: boolean
  timestamp: number
}

let nextId = 0

export function useErrorHandler() {
  const toasts = ref<Toast[]>([])

  function categorize(error: unknown): { message: string; level: 'error' | 'warning' | 'info'; persistent: boolean } {
    const msg = typeof error === 'string' ? error : String(error)

    // Connection errors are persistent
    if (msg.includes('ConnectionFailed') || msg.includes('SessionTimeout') || msg.includes('SecurityRejected')) {
      return { message: msg, level: 'error', persistent: true }
    }
    return { message: msg, level: 'error', persistent: false }
  }

  function handleError(error: unknown) {
    const { message, level, persistent } = categorize(error)
    const toast: Toast = { id: nextId++, message, level, persistent, timestamp: Date.now() }
    toasts.value.push(toast)

    if (!persistent) {
      setTimeout(() => removeToast(toast.id), 3000)
    }
  }

  function removeToast(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id)
  }

  function clearToasts() {
    toasts.value = []
  }

  return { toasts, handleError, removeToast, clearToasts }
}
```

- [ ] **Step 8: Create shared-frontend/src/components/AppDialog.vue**

```vue
<script setup lang="ts">
import { useDialogState, dialogConfirm, dialogCancel } from '../composables/useDialog'
import { nextTick, watch, ref } from 'vue'

const state = useDialogState()
const inputRef = ref<HTMLInputElement | null>(null)

watch(() => state.visible, async (visible) => {
  if (visible && state.mode === 'prompt') {
    await nextTick()
    inputRef.value?.focus()
    inputRef.value?.select()
  }
})

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') dialogConfirm()
  else if (e.key === 'Escape') dialogCancel()
}
</script>

<template>
  <Teleport to="body">
    <div v-if="state.visible" class="dialog-overlay" @keydown="onKeydown">
      <div class="dialog-box">
        <div class="dialog-title">{{ state.title }}</div>
        <div v-if="state.message" class="dialog-message">{{ state.message }}</div>
        <input
          v-if="state.mode === 'prompt'"
          ref="inputRef"
          v-model="state.inputValue"
          class="dialog-input"
          @keydown.enter="dialogConfirm"
        />
        <div class="dialog-buttons">
          <button v-if="state.mode !== 'alert'" class="btn btn-cancel" @click="dialogCancel">Cancel</button>
          <button class="btn btn-confirm" @click="dialogConfirm">OK</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.dialog-box {
  background: #1e1e2e;
  border: 1px solid #313244;
  border-radius: 8px;
  padding: 20px;
  min-width: 320px;
  max-width: 480px;
}

.dialog-title {
  font-size: 14px;
  font-weight: 600;
  color: #cdd6f4;
  margin-bottom: 8px;
}

.dialog-message {
  font-size: 13px;
  color: #a6adc8;
  margin-bottom: 12px;
}

.dialog-input {
  width: 100%;
  padding: 6px 10px;
  background: #11111b;
  border: 1px solid #313244;
  border-radius: 4px;
  color: #cdd6f4;
  font-size: 13px;
  outline: none;
  margin-bottom: 12px;
}

.dialog-input:focus {
  border-color: #89b4fa;
}

.dialog-buttons {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.btn {
  padding: 6px 16px;
  border: none;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
}

.btn-cancel {
  background: #313244;
  color: #cdd6f4;
}

.btn-cancel:hover {
  background: #45475a;
}

.btn-confirm {
  background: #89b4fa;
  color: #1e1e2e;
}

.btn-confirm:hover {
  background: #74c7ec;
}
</style>
```

- [ ] **Step 9: Create shared-frontend/src/index.ts**

```typescript
// Types
export type { LogEntry, DialogMode, DialogState } from './types/common'

// Dialog
export {
  showAlert,
  showConfirm,
  showPrompt,
  dialogConfirm,
  dialogCancel,
  useDialogState,
  dialogKey,
} from './composables/useDialog'

// Log
export { useLogPanel } from './composables/useLogPanel'
export { useLogFilter } from './composables/useLogFilter'
export type { DirectionFilter } from './composables/useLogFilter'

// Error
export { useErrorHandler } from './composables/useErrorHandler'
export type { Toast } from './composables/useErrorHandler'

// Components
export { default as AppDialog } from './components/AppDialog.vue'
```

- [ ] **Step 10: Create master-frontend/package.json**

```json
{
  "name": "master-frontend",
  "private": true,
  "version": "0.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite --port 5178",
    "build": "vue-tsc -b && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "@tanstack/vue-virtual": "^3",
    "@tauri-apps/api": "^2.10.1",
    "@tauri-apps/plugin-dialog": "^2",
    "vue": "^3.5.30",
    "shared-frontend": "*"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.10.1",
    "@types/node": "^24.12.0",
    "@vitejs/plugin-vue": "^6.0.5",
    "@vue/tsconfig": "^0.9.0",
    "typescript": "~5.9.3",
    "vite": "^8.0.1",
    "vue-tsc": "^3.2.5"
  }
}
```

- [ ] **Step 11: Create master-frontend config files**

`master-frontend/vite.config.ts`:
```typescript
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  server: {
    port: 5178,
    strictPort: true,
  },
})
```

`master-frontend/tsconfig.json`:
```json
{
  "files": [],
  "references": [
    { "path": "./tsconfig.app.json" },
    { "path": "./tsconfig.node.json" }
  ]
}
```

`master-frontend/tsconfig.app.json`:
```json
{
  "extends": "@vue/tsconfig/tsconfig.dom.json",
  "compilerOptions": {
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.app.tsbuildinfo",
    "types": ["vite/client"],
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "erasableSyntaxOnly": true,
    "noFallthroughCasesInSwitch": true,
    "noUncheckedSideEffectImports": true
  },
  "include": ["src/**/*.ts", "src/**/*.tsx", "src/**/*.vue"]
}
```

`master-frontend/tsconfig.node.json`:
```json
{
  "compilerOptions": {
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.node.tsbuildinfo",
    "target": "ES2023",
    "lib": ["ES2023"],
    "module": "ESNext",
    "types": ["node"],
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "verbatimModuleSyntax": true,
    "moduleDetection": "force",
    "noEmit": true,
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "erasableSyntaxOnly": true,
    "noFallthroughCasesInSwitch": true,
    "noUncheckedSideEffectImports": true
  },
  "include": ["vite.config.ts"]
}
```

`master-frontend/index.html`:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>OPCUAMaster</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 12: Create master-frontend/src/main.ts**

```typescript
import { createApp } from 'vue'
import App from './App.vue'

createApp(App).mount('#app')
```

- [ ] **Step 13: Create master-frontend/src/types.ts**

```typescript
export interface ConnectionInfo {
  id: string
  name: string
  endpoint_url: string
  security_policy: string
  security_mode: string
  auth_type: string
  state: string // 'Disconnected' | 'Connecting' | 'Connected' | 'Reconnecting'
}

export interface BrowseResult {
  node_id: string
  display_name: string
  node_class: string
  data_type?: string
  has_children: boolean
}

export interface MonitoredNodeInfo {
  node_id: string
  display_name: string
  browse_path: string
  data_type: string
  value?: string
  quality?: string
  timestamp?: string
  access_mode: string  // 'Subscription' | 'Polling'
  interval_ms: number
  group_id?: string
}

export interface NodeGroupInfo {
  id: string
  name: string
  node_count: number
}

export interface NodeAttributesInfo {
  node_id: string
  display_name: string
  description: string
  data_type: string
  access_level: string
  value?: string
  quality?: string
  timestamp?: string
}

export interface ConnectionStateEvent {
  id: string
  state: string
}

export interface DataChangedEvent {
  connection_id: string
  items: DataChangeItem[]
}

export interface DataChangeItem {
  node_id: string
  value: string
  quality: string
  timestamp: string
}
```

- [ ] **Step 14: Create master-frontend/src/composables/useDialog.ts**

```typescript
export {
  showAlert,
  showConfirm,
  showPrompt,
  dialogConfirm,
  dialogCancel,
  useDialogState,
  dialogKey,
} from 'shared-frontend'
export type { DialogMode, DialogState } from 'shared-frontend'
```

- [ ] **Step 15: Create master-frontend/src/App.vue (shell layout)**

```vue
<script setup lang="ts">
import { ref, provide, onMounted, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { AppDialog } from 'shared-frontend'
import { showAlert, showConfirm, showPrompt, dialogKey } from './composables/useDialog'
import type { ConnectionStateEvent, MonitoredNodeInfo } from './types'

// Shared state
const selectedConnectionId = ref<string | null>(null)
const selectedConnectionState = ref<string>('Disconnected')
const selectedNodes = ref<MonitoredNodeInfo[]>([])
const logExpanded = ref(false)

// Provide shared state
provide('selectedConnectionId', selectedConnectionId)
provide('selectedConnectionState', selectedConnectionState)
provide('selectedNodes', selectedNodes)
provide(dialogKey, { showAlert, showConfirm, showPrompt })

// Refresh triggers
const treeRefreshKey = ref(0)
provide('treeRefreshKey', treeRefreshKey)
function refreshTree() { treeRefreshKey.value++ }
provide('refreshTree', refreshTree)

const dataRefreshKey = ref(0)
provide('dataRefreshKey', dataRefreshKey)
function refreshData() { dataRefreshKey.value++ }
provide('refreshData', refreshData)

// Backend event listeners
let unlistenConnState: (() => void) | null = null
let unlistenDataChanged: (() => void) | null = null

onMounted(async () => {
  unlistenConnState = await listen<ConnectionStateEvent>('connection-state-changed', (event) => {
    const { id, state } = event.payload
    if (selectedConnectionId.value === id) {
      selectedConnectionState.value = state
    }
    refreshTree()
  })
  unlistenDataChanged = await listen('data-changed', () => {
    refreshData()
  })
})

onUnmounted(() => {
  unlistenConnState?.()
  unlistenDataChanged?.()
})

function handleConnectionSelect(id: string, state: string) {
  selectedConnectionId.value = id
  selectedConnectionState.value = state
  selectedNodes.value = []
}

function handleNodeSelect(nodes: MonitoredNodeInfo[]) {
  selectedNodes.value = nodes
}

function toggleLog() {
  logExpanded.value = !logExpanded.value
}
</script>

<template>
  <div :class="['app-layout', { 'log-expanded': logExpanded }]">
    <header class="toolbar-area">
      <div style="padding: 8px 12px; font-size: 13px; color: #a6adc8;">OPCUAMaster — Toolbar placeholder</div>
    </header>

    <aside class="tree-area">
      <div style="padding: 8px 12px; font-size: 13px; color: #a6adc8;">Connection tree placeholder</div>
    </aside>
    <main class="content-area">
      <div style="padding: 8px 12px; font-size: 13px; color: #a6adc8;">Data table placeholder</div>
    </main>
    <aside class="panel-area">
      <div style="padding: 8px 12px; font-size: 13px; color: #a6adc8;">Value panel placeholder</div>
    </aside>

    <footer class="log-area">
      <div style="padding: 8px 12px; font-size: 13px; color: #a6adc8; cursor: pointer;" @click="toggleLog">
        Log panel placeholder (click to toggle)
      </div>
    </footer>
    <AppDialog />
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body, #app {
  height: 100%;
  width: 100%;
  overflow: hidden;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
  background: #11111b;
  color: #cdd6f4;
}

.app-layout {
  display: grid;
  grid-template-columns: 260px 1fr 280px;
  grid-template-rows: 42px 1fr 32px;
  grid-template-areas:
    "toolbar toolbar toolbar"
    "tree content panel"
    "log log log";
  height: 100vh;
  width: 100vw;
}

.app-layout.log-expanded {
  grid-template-rows: 42px 1fr 200px;
}

.toolbar-area {
  grid-area: toolbar;
  background: #1e1e2e;
  border-bottom: 1px solid #313244;
}

.tree-area {
  grid-area: tree;
  background: #181825;
  border-right: 1px solid #313244;
  overflow-y: auto;
}

.content-area {
  grid-area: content;
  background: #11111b;
  overflow: hidden;
}

.panel-area {
  grid-area: panel;
  background: #181825;
  border-left: 1px solid #313244;
  overflow-y: auto;
}

.log-area {
  grid-area: log;
  background: #1e1e2e;
  border-top: 1px solid #313244;
  overflow: hidden;
}
</style>
```

- [ ] **Step 16: Install npm dependencies and verify**

Run: `npm install`
Expected: Installs all dependencies for shared-frontend and master-frontend

Run: `cd master-frontend && npm run build`
Expected: Builds successfully (or may need `npx vue-tsc -b` fix)

- [ ] **Step 17: Verify full Tauri dev launch**

Run: `cd crates/opcuamaster-app && cargo tauri dev`
Expected: App window opens with placeholder layout, Catppuccin dark theme visible

- [ ] **Step 18: Commit**

```bash
git add shared-frontend/ master-frontend/
git commit -m "feat: scaffold shared-frontend and master-frontend with grid layout"
```

---

## Task 3: Core Library — Error, LogEntry, LogCollector, Reconnect

**Files:**
- Create: `crates/opcuasim-core/src/error.rs`
- Create: `crates/opcuasim-core/src/log_entry.rs`
- Create: `crates/opcuasim-core/src/log_collector.rs`
- Create: `crates/opcuasim-core/src/reconnect.rs`

- [ ] **Step 1: Create error.rs**

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Clone)]
pub enum OpcUaSimError {
    // Connection layer
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Session timeout")]
    SessionTimeout,
    #[error("Security rejected: {0}")]
    SecurityRejected(String),
    #[error("Authentication failed")]
    AuthenticationFailed,

    // Protocol layer
    #[error("Browse error: {0}")]
    BrowseError(String),
    #[error("Read error: {0}")]
    ReadError(String),
    #[error("Write error: {0}")]
    WriteError(String),
    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    // Application layer
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("Project file error: {0}")]
    ProjectFileError(String),
    #[error("Output error: {0}")]
    OutputError(String),

    // Generic
    #[error("IO error: {0}")]
    Io(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl OpcUaSimError {
    pub fn category(&self) -> &'static str {
        match self {
            Self::ConnectionFailed(_) | Self::SessionTimeout
            | Self::SecurityRejected(_) | Self::AuthenticationFailed => "connection",
            Self::BrowseError(_) | Self::ReadError(_)
            | Self::WriteError(_) | Self::SubscriptionError(_) => "protocol",
            Self::ConfigError(_) | Self::ProjectFileError(_)
            | Self::OutputError(_) => "application",
            Self::Io(_) | Self::Internal(_) => "generic",
        }
    }
}

impl From<std::io::Error> for OpcUaSimError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
```

- [ ] **Step 2: Create log_entry.rs**

```rust
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum Direction {
    Request,
    Response,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Request => write!(f, "Request"),
            Direction::Response => write!(f, "Response"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub seq: u64,
    pub timestamp: DateTime<Utc>,
    pub connection_id: String,
    pub direction: Direction,
    pub service: String,
    pub detail: String,
    pub status: Option<String>,
}

impl LogEntry {
    pub fn new(
        seq: u64,
        connection_id: String,
        direction: Direction,
        service: String,
        detail: String,
        status: Option<String>,
    ) -> Self {
        Self {
            seq,
            timestamp: Utc::now(),
            connection_id,
            direction,
            service,
            detail,
            status,
        }
    }

    pub fn to_csv_row(&self) -> String {
        format!(
            "{},{},{},{},{},{}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            self.direction,
            self.service,
            self.detail.replace(',', ";"),
            self.status.as_deref().unwrap_or(""),
            self.connection_id,
        )
    }

    pub fn csv_header() -> &'static str {
        "Timestamp,Direction,Service,Detail,Status,ConnectionId"
    }
}
```

- [ ] **Step 3: Create log_collector.rs**

```rust
use std::sync::{Arc, RwLock};
use crate::log_entry::LogEntry;

const MAX_LOG_ENTRIES: usize = 10_000;

#[derive(Clone)]
pub struct LogCollector {
    entries: Arc<RwLock<Vec<LogEntry>>>,
    seq_counter: Arc<RwLock<u64>>,
}

impl LogCollector {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            seq_counter: Arc::new(RwLock::new(0)),
        }
    }

    pub fn next_seq(&self) -> u64 {
        let mut counter = self.seq_counter.write().unwrap();
        *counter += 1;
        *counter
    }

    pub fn add(&self, entry: LogEntry) {
        let mut entries = self.entries.write().unwrap();
        if entries.len() >= MAX_LOG_ENTRIES {
            entries.remove(0);
        }
        entries.push(entry);
    }

    pub fn get_since(&self, since_seq: u64) -> Vec<LogEntry> {
        let entries = self.entries.read().unwrap();
        entries.iter().filter(|e| e.seq > since_seq).cloned().collect()
    }

    pub fn get_all(&self) -> Vec<LogEntry> {
        self.entries.read().unwrap().clone()
    }

    pub fn clear(&self) {
        self.entries.write().unwrap().clear();
    }

    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn export_csv(&self) -> String {
        let entries = self.entries.read().unwrap();
        let mut csv = String::from(LogEntry::csv_header());
        csv.push('\n');
        for entry in entries.iter() {
            csv.push_str(&entry.to_csv_row());
            csv.push('\n');
        }
        csv
    }
}

impl Default for LogCollector {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Create reconnect.rs**

```rust
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ReconnectPolicy {
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f64,
    pub max_attempts: Option<u32>,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial_delay_ms: 1000,
            max_delay_ms: 60_000,
            backoff_factor: 2.0,
            max_attempts: None,
        }
    }
}

impl ReconnectPolicy {
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay_ms as f64 * self.backoff_factor.powi(attempt as i32);
        let clamped = delay.min(self.max_delay_ms as f64) as u64;
        Duration::from_millis(clamped)
    }

    pub fn should_retry(&self, attempt: u32) -> bool {
        match self.max_attempts {
            Some(max) => attempt < max,
            None => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReconnectState {
    Idle,
    Reconnecting { attempt: u32 },
    GaveUp,
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p opcuasim-core`
Expected: Compiles successfully

- [ ] **Step 6: Write unit tests**

Add to bottom of `crates/opcuasim-core/src/log_collector.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_entry::Direction;

    fn make_entry(collector: &LogCollector, service: &str) -> LogEntry {
        LogEntry::new(
            collector.next_seq(),
            "test-conn".to_string(),
            Direction::Request,
            service.to_string(),
            "test detail".to_string(),
            None,
        )
    }

    #[test]
    fn test_add_and_get_all() {
        let collector = LogCollector::new();
        collector.add(make_entry(&collector, "Browse"));
        collector.add(make_entry(&collector, "Read"));
        assert_eq!(collector.len(), 2);
        let all = collector.get_all();
        assert_eq!(all[0].service, "Browse");
        assert_eq!(all[1].service, "Read");
    }

    #[test]
    fn test_get_since() {
        let collector = LogCollector::new();
        collector.add(make_entry(&collector, "Browse"));
        collector.add(make_entry(&collector, "Read"));
        let since = collector.get_since(1);
        assert_eq!(since.len(), 1);
        assert_eq!(since[0].service, "Read");
    }

    #[test]
    fn test_clear() {
        let collector = LogCollector::new();
        collector.add(make_entry(&collector, "Browse"));
        collector.clear();
        assert!(collector.is_empty());
    }

    #[test]
    fn test_csv_export() {
        let collector = LogCollector::new();
        collector.add(make_entry(&collector, "Browse"));
        let csv = collector.export_csv();
        assert!(csv.starts_with("Timestamp,"));
        assert!(csv.contains("Browse"));
    }
}
```

Add to bottom of `crates/opcuasim-core/src/reconnect.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let policy = ReconnectPolicy::default();
        assert_eq!(policy.delay_for_attempt(0), Duration::from_millis(1000));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(2000));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(4000));
    }

    #[test]
    fn test_max_delay_cap() {
        let policy = ReconnectPolicy::default();
        // attempt 10 → 1000 * 2^10 = 1,024,000 → capped to 60,000
        assert_eq!(policy.delay_for_attempt(10), Duration::from_millis(60_000));
    }

    #[test]
    fn test_should_retry_unlimited() {
        let policy = ReconnectPolicy::default();
        assert!(policy.should_retry(100));
    }

    #[test]
    fn test_should_retry_limited() {
        let policy = ReconnectPolicy {
            max_attempts: Some(3),
            ..Default::default()
        };
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
    }
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p opcuasim-core`
Expected: All tests pass

- [ ] **Step 8: Commit**

```bash
git add crates/opcuasim-core/src/
git commit -m "feat: add error types, log collector, and reconnect policy for opcuasim-core"
```

---

## Task 4: Core Library — Node, Config, Output

**Files:**
- Create: `crates/opcuasim-core/src/node.rs`
- Create: `crates/opcuasim-core/src/config.rs`
- Create: `crates/opcuasim-core/src/output.rs`

- [ ] **Step 1: Create node.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessMode {
    Subscription { interval_ms: f64 },
    Polling { interval_ms: u64 },
}

impl Default for AccessMode {
    fn default() -> Self {
        AccessMode::Subscription { interval_ms: 1000.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoredNode {
    pub node_id: String,
    pub display_name: String,
    pub browse_path: String,
    pub data_type: String,
    pub value: Option<String>,
    pub quality: Option<String>,
    pub timestamp: Option<String>,
    pub access_mode: AccessMode,
    pub group_id: Option<String>,
    pub update_seq: u64,
}

impl MonitoredNode {
    pub fn new(node_id: String, display_name: String, browse_path: String, data_type: String) -> Self {
        Self {
            node_id,
            display_name,
            browse_path,
            data_type,
            value: None,
            quality: None,
            timestamp: None,
            access_mode: AccessMode::default(),
            group_id: None,
            update_seq: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGroup {
    pub id: String,
    pub name: String,
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseResultItem {
    pub node_id: String,
    pub display_name: String,
    pub node_class: String,
    pub data_type: Option<String>,
    pub has_children: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAttributes {
    pub node_id: String,
    pub display_name: String,
    pub description: String,
    pub data_type: String,
    pub access_level: String,
    pub value: Option<String>,
    pub quality: Option<String>,
    pub timestamp: Option<String>,
}
```

- [ ] **Step 2: Create config.rs**

```rust
use serde::{Deserialize, Serialize};
use crate::node::{AccessMode, NodeGroup};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthConfig {
    Anonymous,
    UserPassword { username: String, password: String },
    Certificate { cert_path: String, key_path: String },
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig::Anonymous
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub endpoint_url: String,
    pub security_policy: String,
    pub security_mode: String,
    pub auth: AuthConfig,
    pub timeout_ms: u64,
}

impl ConnectionConfig {
    pub fn new(id: String, name: String, endpoint_url: String) -> Self {
        Self {
            id,
            name,
            endpoint_url,
            security_policy: "None".to_string(),
            security_mode: "None".to_string(),
            auth: AuthConfig::default(),
            timeout_ms: 5000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoredNodeConfig {
    pub node_id: String,
    pub display_name: String,
    pub access_mode: AccessMode,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProjectEntry {
    pub name: String,
    pub endpoint_url: String,
    pub security_policy: String,
    pub security_mode: String,
    pub auth: AuthConfig,
    pub timeout_ms: u64,
    pub monitored_nodes: Vec<MonitoredNodeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    #[serde(rename = "type")]
    pub project_type: String,
    pub version: String,
    pub connections: Vec<ConnectionProjectEntry>,
    pub groups: Vec<NodeGroup>,
}

impl ProjectFile {
    pub fn new_master() -> Self {
        Self {
            project_type: "OpcUaMaster".to_string(),
            version: "0.1.0".to_string(),
            connections: vec![],
            groups: vec![],
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
```

- [ ] **Step 3: Create output.rs**

```rust
use crate::log_collector::LogCollector;
use crate::log_entry::{Direction, LogEntry};

pub struct DataChangeItem {
    pub node_id: String,
    pub value: String,
    pub quality: String,
    pub timestamp: String,
}

#[allow(async_fn_in_trait)]
pub trait DataOutput: Send + Sync {
    async fn on_data_change(&self, connection_id: &str, items: &[DataChangeItem]);
    async fn on_connect(&self, connection_id: &str);
    async fn on_disconnect(&self, connection_id: &str);
}

pub struct LogOutput {
    collector: LogCollector,
}

impl LogOutput {
    pub fn new(collector: LogCollector) -> Self {
        Self { collector }
    }
}

impl DataOutput for LogOutput {
    async fn on_data_change(&self, connection_id: &str, items: &[DataChangeItem]) {
        for item in items {
            let seq = self.collector.next_seq();
            self.collector.add(LogEntry::new(
                seq,
                connection_id.to_string(),
                Direction::Response,
                "DataChange".to_string(),
                format!("{} = {} [{}]", item.node_id, item.value, item.quality),
                None,
            ));
        }
    }

    async fn on_connect(&self, connection_id: &str) {
        let seq = self.collector.next_seq();
        self.collector.add(LogEntry::new(
            seq,
            connection_id.to_string(),
            Direction::Response,
            "Session".to_string(),
            "Connected".to_string(),
            None,
        ));
    }

    async fn on_disconnect(&self, connection_id: &str) {
        let seq = self.collector.next_seq();
        self.collector.add(LogEntry::new(
            seq,
            connection_id.to_string(),
            Direction::Response,
            "Session".to_string(),
            "Disconnected".to_string(),
            None,
        ));
    }
}
```

- [ ] **Step 4: Write tests for config serialization**

Add to bottom of `crates/opcuasim-core/src/config.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_file_roundtrip() {
        let mut project = ProjectFile::new_master();
        project.connections.push(ConnectionProjectEntry {
            name: "Test".to_string(),
            endpoint_url: "opc.tcp://localhost:4840".to_string(),
            security_policy: "None".to_string(),
            security_mode: "None".to_string(),
            auth: AuthConfig::Anonymous,
            timeout_ms: 5000,
            monitored_nodes: vec![MonitoredNodeConfig {
                node_id: "ns=2;s=Temperature".to_string(),
                display_name: "Temperature".to_string(),
                access_mode: AccessMode::Subscription { interval_ms: 1000.0 },
                group_id: None,
            }],
        });
        project.groups.push(NodeGroup {
            id: "g1".to_string(),
            name: "Group 1".to_string(),
            node_ids: vec!["ns=2;s=Temperature".to_string()],
        });

        let json = project.to_json().unwrap();
        let parsed = ProjectFile::from_json(&json).unwrap();
        assert_eq!(parsed.project_type, "OpcUaMaster");
        assert_eq!(parsed.connections.len(), 1);
        assert_eq!(parsed.connections[0].monitored_nodes[0].node_id, "ns=2;s=Temperature");
        assert_eq!(parsed.groups.len(), 1);
    }

    #[test]
    fn test_auth_config_variants() {
        let anon = serde_json::to_string(&AuthConfig::Anonymous).unwrap();
        assert!(anon.contains("Anonymous"));

        let user = serde_json::to_string(&AuthConfig::UserPassword {
            username: "admin".to_string(),
            password: "pass".to_string(),
        }).unwrap();
        assert!(user.contains("admin"));
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p opcuasim-core`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/opcuasim-core/src/node.rs crates/opcuasim-core/src/config.rs crates/opcuasim-core/src/output.rs
git commit -m "feat: add node types, config serialization, and output plugin trait"
```

---

## Task 5: Core Library — OPC UA Client Connection Manager

**Files:**
- Create: `crates/opcuasim-core/src/client.rs`

- [ ] **Step 1: Create client.rs**

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, error, warn};

use crate::config::{AuthConfig, ConnectionConfig};
use crate::error::OpcUaSimError;
use crate::log_collector::LogCollector;
use crate::log_entry::{Direction, LogEntry};
use crate::reconnect::{ReconnectPolicy, ReconnectState};

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "Disconnected"),
            ConnectionState::Connecting => write!(f, "Connecting"),
            ConnectionState::Connected => write!(f, "Connected"),
            ConnectionState::Reconnecting => write!(f, "Reconnecting"),
        }
    }
}

impl serde::Serialize for ConnectionState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub struct OpcUaConnection {
    pub config: ConnectionConfig,
    pub state: Arc<RwLock<ConnectionState>>,
    pub log_collector: LogCollector,
    reconnect_policy: ReconnectPolicy,
    reconnect_state: Arc<RwLock<ReconnectState>>,
    shutdown_tx: Arc<RwLock<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl OpcUaConnection {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            log_collector: LogCollector::new(),
            reconnect_policy: ReconnectPolicy::default(),
            reconnect_state: Arc::new(RwLock::new(ReconnectState::Idle)),
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }

    async fn set_state(&self, new_state: ConnectionState) {
        let mut state = self.state.write().await;
        *state = new_state;
    }

    fn log_request(&self, service: &str, detail: &str) {
        let seq = self.log_collector.next_seq();
        self.log_collector.add(LogEntry::new(
            seq,
            self.config.id.clone(),
            Direction::Request,
            service.to_string(),
            detail.to_string(),
            None,
        ));
    }

    fn log_response(&self, service: &str, detail: &str, status: Option<&str>) {
        let seq = self.log_collector.next_seq();
        self.log_collector.add(LogEntry::new(
            seq,
            self.config.id.clone(),
            Direction::Response,
            service.to_string(),
            detail.to_string(),
            status.map(|s| s.to_string()),
        ));
    }

    pub async fn connect(&self) -> Result<(), OpcUaSimError> {
        self.set_state(ConnectionState::Connecting).await;
        self.log_request("Session", &format!("Connecting to {}", self.config.endpoint_url));

        // TODO: Task 8 will implement actual async-opcua session creation here.
        // For now, simulate a successful connection.
        info!("Connecting to OPC UA server: {}", self.config.endpoint_url);

        self.set_state(ConnectionState::Connected).await;
        self.log_response("Session", "Connected", Some("Good"));
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), OpcUaSimError> {
        // Send shutdown signal if background tasks running
        let mut tx_guard = self.shutdown_tx.write().await;
        if let Some(tx) = tx_guard.take() {
            let _ = tx.send(());
        }

        self.set_state(ConnectionState::Disconnected).await;
        self.log_request("Session", "Disconnecting");
        self.log_response("Session", "Disconnected", Some("Good"));
        info!("Disconnected from: {}", self.config.endpoint_url);
        Ok(())
    }

    pub async fn start_reconnect_loop<F>(&self, on_state_change: F)
    where
        F: Fn(ConnectionState) + Send + Sync + 'static,
    {
        let state = self.state.clone();
        let reconnect_state = self.reconnect_state.clone();
        let policy = self.reconnect_policy.clone();
        let endpoint = self.config.endpoint_url.clone();

        let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
        {
            let mut tx_guard = self.shutdown_tx.write().await;
            *tx_guard = Some(tx);
        }

        tokio::spawn(async move {
            let mut attempt: u32 = 0;
            loop {
                if !policy.should_retry(attempt) {
                    *reconnect_state.write().await = ReconnectState::GaveUp;
                    warn!("Gave up reconnecting to {}", endpoint);
                    break;
                }

                *reconnect_state.write().await = ReconnectState::Reconnecting { attempt };
                *state.write().await = ConnectionState::Reconnecting;
                on_state_change(ConnectionState::Reconnecting);

                let delay = policy.delay_for_attempt(attempt);
                tokio::select! {
                    _ = tokio::time::sleep(delay) => {}
                    _ = &mut rx => {
                        info!("Reconnect loop cancelled");
                        return;
                    }
                }

                // TODO: Task 8 will implement actual reconnection attempt.
                // For now, just log the attempt.
                info!("Reconnect attempt {} to {}", attempt + 1, endpoint);
                attempt += 1;
            }
        });
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p opcuasim-core`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add crates/opcuasim-core/src/client.rs
git commit -m "feat: add OPC UA connection manager with state tracking and reconnect loop"
```

---

## Task 6: Core Library — Browse + Subscription + Polling (Stubs)

**Files:**
- Create: `crates/opcuasim-core/src/browse.rs`
- Create: `crates/opcuasim-core/src/subscription.rs`
- Create: `crates/opcuasim-core/src/polling.rs`

- [ ] **Step 1: Create browse.rs**

```rust
use crate::error::OpcUaSimError;
use crate::node::{BrowseResultItem, NodeAttributes};

/// Browse children of a node. Pass None for node_id to browse from root (Objects folder).
pub async fn browse_node(
    _endpoint_url: &str,
    _node_id: Option<&str>,
) -> Result<Vec<BrowseResultItem>, OpcUaSimError> {
    // TODO: Task 8 will implement actual async-opcua browsing.
    // Return empty for now — the Tauri command layer will call this.
    Ok(vec![])
}

/// Read detailed attributes of a specific node.
pub async fn read_node_attributes(
    _endpoint_url: &str,
    _node_id: &str,
) -> Result<NodeAttributes, OpcUaSimError> {
    // TODO: Task 8 will implement actual async-opcua attribute reading.
    Err(OpcUaSimError::ReadError("Not yet implemented".to_string()))
}
```

- [ ] **Step 2: Create subscription.rs**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::info;

use crate::error::OpcUaSimError;
use crate::node::MonitoredNode;
use crate::output::DataChangeItem;

pub struct SubscriptionManager {
    /// node_id → MonitoredNode
    monitored_items: Arc<RwLock<HashMap<String, MonitoredNode>>>,
    update_seq: Arc<RwLock<u64>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            monitored_items: Arc::new(RwLock::new(HashMap::new())),
            update_seq: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn add_nodes(&self, nodes: Vec<MonitoredNode>) -> Result<(), OpcUaSimError> {
        let mut items = self.monitored_items.write().await;
        for node in nodes {
            info!("Adding subscription for node: {}", node.node_id);
            items.insert(node.node_id.clone(), node);
        }
        // TODO: Task 8 will create actual OPC UA monitored items.
        Ok(())
    }

    pub async fn remove_nodes(&self, node_ids: &[String]) -> Result<(), OpcUaSimError> {
        let mut items = self.monitored_items.write().await;
        for id in node_ids {
            items.remove(id);
        }
        // TODO: Task 8 will remove actual OPC UA monitored items.
        Ok(())
    }

    pub async fn get_monitored_nodes(&self) -> Vec<MonitoredNode> {
        self.monitored_items.read().await.values().cloned().collect()
    }

    pub async fn get_monitored_nodes_since(&self, since_seq: u64) -> Vec<MonitoredNode> {
        self.monitored_items
            .read()
            .await
            .values()
            .filter(|n| n.update_seq > since_seq)
            .cloned()
            .collect()
    }

    pub async fn apply_data_changes(&self, items: &[DataChangeItem]) {
        let mut monitored = self.monitored_items.write().await;
        let mut seq = self.update_seq.write().await;
        for item in items {
            if let Some(node) = monitored.get_mut(&item.node_id) {
                *seq += 1;
                node.value = Some(item.value.clone());
                node.quality = Some(item.quality.clone());
                node.timestamp = Some(item.timestamp.clone());
                node.update_seq = *seq;
            }
        }
    }

    pub async fn get_update_seq(&self) -> u64 {
        *self.update_seq.read().await
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: Create polling.rs**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use log::info;

use crate::error::OpcUaSimError;
use crate::node::MonitoredNode;

pub struct PollingManager {
    /// node_id → polling interval in ms
    polling_tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    monitored_items: Arc<RwLock<HashMap<String, MonitoredNode>>>,
}

impl PollingManager {
    pub fn new() -> Self {
        Self {
            polling_tasks: Arc::new(RwLock::new(HashMap::new())),
            monitored_items: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_polling_node(&self, node: MonitoredNode, interval_ms: u64) -> Result<(), OpcUaSimError> {
        let node_id = node.node_id.clone();
        info!("Adding polling for node: {} (interval: {}ms)", node_id, interval_ms);

        {
            let mut items = self.monitored_items.write().await;
            items.insert(node_id.clone(), node);
        }

        let items = self.monitored_items.clone();
        let nid = node_id.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(interval_ms));
            loop {
                interval.tick().await;
                // TODO: Task 8 will implement actual OPC UA read here
                // and update items[nid] with the result.
                let _items = items.read().await;
                if !_items.contains_key(&nid) {
                    break;
                }
            }
        });

        let mut tasks = self.polling_tasks.write().await;
        if let Some(old_handle) = tasks.insert(node_id, handle) {
            old_handle.abort();
        }

        Ok(())
    }

    pub async fn remove_polling_node(&self, node_id: &str) {
        let mut tasks = self.polling_tasks.write().await;
        if let Some(handle) = tasks.remove(node_id) {
            handle.abort();
        }
        let mut items = self.monitored_items.write().await;
        items.remove(node_id);
    }

    pub async fn stop_all(&self) {
        let mut tasks = self.polling_tasks.write().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }
    }

    pub async fn get_polling_nodes(&self) -> Vec<MonitoredNode> {
        self.monitored_items.read().await.values().cloned().collect()
    }
}

impl Default for PollingManager {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p opcuasim-core`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add crates/opcuasim-core/src/browse.rs crates/opcuasim-core/src/subscription.rs crates/opcuasim-core/src/polling.rs
git commit -m "feat: add browse, subscription, and polling managers (stubs for async-opcua)"
```

---

## Task 7: Tauri App — State, Commands, and Event Wiring

**Files:**
- Modify: `crates/opcuamaster-app/src/state.rs`
- Modify: `crates/opcuamaster-app/src/commands.rs`
- Modify: `crates/opcuamaster-app/src/lib.rs`

- [ ] **Step 1: Rewrite state.rs with full connection state**

```rust
use std::collections::HashMap;
use std::sync::RwLock;
use opcuasim_core::client::OpcUaConnection;
use opcuasim_core::node::NodeGroup;
use serde::Serialize;

pub struct ConnectionEntry {
    pub connection: OpcUaConnection,
}

pub struct AppState {
    pub connections: RwLock<HashMap<String, ConnectionEntry>>,
    pub groups: RwLock<Vec<NodeGroup>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            groups: RwLock::new(Vec::new()),
        }
    }
}

// DTOs for frontend

#[derive(Serialize)]
pub struct ConnectionInfoDto {
    pub id: String,
    pub name: String,
    pub endpoint_url: String,
    pub security_policy: String,
    pub security_mode: String,
    pub auth_type: String,
    pub state: String,
}

#[derive(Serialize)]
pub struct BrowseResultDto {
    pub node_id: String,
    pub display_name: String,
    pub node_class: String,
    pub data_type: Option<String>,
    pub has_children: bool,
}

#[derive(Serialize)]
pub struct MonitoredNodeDto {
    pub node_id: String,
    pub display_name: String,
    pub browse_path: String,
    pub data_type: String,
    pub value: Option<String>,
    pub quality: Option<String>,
    pub timestamp: Option<String>,
    pub access_mode: String,
    pub interval_ms: f64,
    pub group_id: Option<String>,
}

#[derive(Serialize)]
pub struct NodeGroupDto {
    pub id: String,
    pub name: String,
    pub node_count: usize,
}

#[derive(Clone, Serialize)]
pub struct ConnectionStateEvent {
    pub id: String,
    pub state: String,
}

#[derive(Clone, Serialize)]
pub struct DataChangedEvent {
    pub connection_id: String,
    pub items: Vec<DataChangeItemDto>,
}

#[derive(Clone, Serialize)]
pub struct DataChangeItemDto {
    pub node_id: String,
    pub value: String,
    pub quality: String,
    pub timestamp: String,
}
```

- [ ] **Step 2: Rewrite commands.rs with connection CRUD + log + project commands**

```rust
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use opcuasim_core::client::OpcUaConnection;
use opcuasim_core::config::{AuthConfig, ConnectionConfig, ProjectFile, ConnectionProjectEntry, MonitoredNodeConfig};
use opcuasim_core::node::{AccessMode, MonitoredNode, NodeGroup};

use crate::state::{
    AppState, ConnectionEntry, ConnectionInfoDto, ConnectionStateEvent,
    MonitoredNodeDto, NodeGroupDto,
};

// ── Connection Management ──────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct CreateConnectionRequest {
    pub name: String,
    pub endpoint_url: String,
    pub security_policy: Option<String>,
    pub security_mode: Option<String>,
    pub auth_type: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub timeout_ms: Option<u64>,
}

#[tauri::command]
pub fn create_connection(
    state: State<'_, AppState>,
    request: CreateConnectionRequest,
) -> Result<ConnectionInfoDto, String> {
    let id = Uuid::new_v4().to_string();

    let auth = match request.auth_type.as_deref() {
        Some("UserPassword") => AuthConfig::UserPassword {
            username: request.username.unwrap_or_default(),
            password: request.password.unwrap_or_default(),
        },
        Some("Certificate") => AuthConfig::Certificate {
            cert_path: request.cert_path.unwrap_or_default(),
            key_path: request.key_path.unwrap_or_default(),
        },
        _ => AuthConfig::Anonymous,
    };

    let config = ConnectionConfig {
        id: id.clone(),
        name: request.name.clone(),
        endpoint_url: request.endpoint_url.clone(),
        security_policy: request.security_policy.unwrap_or_else(|| "None".to_string()),
        security_mode: request.security_mode.unwrap_or_else(|| "None".to_string()),
        auth,
        timeout_ms: request.timeout_ms.unwrap_or(5000),
    };

    let auth_type = match &config.auth {
        AuthConfig::Anonymous => "Anonymous",
        AuthConfig::UserPassword { .. } => "UserPassword",
        AuthConfig::Certificate { .. } => "Certificate",
    };

    let dto = ConnectionInfoDto {
        id: id.clone(),
        name: config.name.clone(),
        endpoint_url: config.endpoint_url.clone(),
        security_policy: config.security_policy.clone(),
        security_mode: config.security_mode.clone(),
        auth_type: auth_type.to_string(),
        state: "Disconnected".to_string(),
    };

    let connection = OpcUaConnection::new(config);
    let mut connections = state.connections.write().map_err(|e| e.to_string())?;
    connections.insert(id, ConnectionEntry { connection });

    Ok(dto)
}

#[tauri::command]
pub async fn connect(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let connection = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        let entry = connections.get(&id).ok_or("Connection not found")?;
        // We need to get info to use outside the lock
        entry.connection.config.id.clone()
    };

    // Get a reference to the connection to call connect
    let connections = state.connections.read().map_err(|e| e.to_string())?;
    let entry = connections.get(&id).ok_or("Connection not found")?;

    entry.connection.connect().await.map_err(|e| e.to_string())?;

    let _ = app.emit("connection-state-changed", ConnectionStateEvent {
        id: connection,
        state: "Connected".to_string(),
    });

    Ok(())
}

#[tauri::command]
pub async fn disconnect(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let connections = state.connections.read().map_err(|e| e.to_string())?;
    let entry = connections.get(&id).ok_or("Connection not found")?;

    entry.connection.disconnect().await.map_err(|e| e.to_string())?;

    let _ = app.emit("connection-state-changed", ConnectionStateEvent {
        id: id.clone(),
        state: "Disconnected".to_string(),
    });

    Ok(())
}

#[tauri::command]
pub fn delete_connection(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut connections = state.connections.write().map_err(|e| e.to_string())?;
    connections.remove(&id).ok_or("Connection not found")?;
    Ok(())
}

#[tauri::command]
pub async fn list_connections(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionInfoDto>, String> {
    let connections = state.connections.read().map_err(|e| e.to_string())?;
    let mut result = Vec::new();

    for (id, entry) in connections.iter() {
        let conn_state = entry.connection.get_state().await;
        let auth_type = match &entry.connection.config.auth {
            AuthConfig::Anonymous => "Anonymous",
            AuthConfig::UserPassword { .. } => "UserPassword",
            AuthConfig::Certificate { .. } => "Certificate",
        };
        result.push(ConnectionInfoDto {
            id: id.clone(),
            name: entry.connection.config.name.clone(),
            endpoint_url: entry.connection.config.endpoint_url.clone(),
            security_policy: entry.connection.config.security_policy.clone(),
            security_mode: entry.connection.config.security_mode.clone(),
            auth_type: auth_type.to_string(),
            state: conn_state.to_string(),
        });
    }

    Ok(result)
}

// ── Endpoint Discovery ──────────────────────────────────────────

#[tauri::command]
pub async fn get_endpoints(
    url: String,
) -> Result<Vec<String>, String> {
    // TODO: Task 8 will implement actual endpoint discovery via async-opcua.
    // Returns list of endpoint URLs with their security policies.
    Ok(vec![url])
}

// ── Log Commands ──────────────────────────────────────────

#[tauri::command]
pub fn get_communication_logs(
    state: State<'_, AppState>,
    conn_id: String,
    since_seq: u64,
) -> Result<Vec<opcuasim_core::log_entry::LogEntry>, String> {
    let connections = state.connections.read().map_err(|e| e.to_string())?;
    let entry = connections.get(&conn_id).ok_or("Connection not found")?;
    Ok(entry.connection.log_collector.get_since(since_seq))
}

#[tauri::command]
pub fn clear_communication_logs(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<(), String> {
    let connections = state.connections.read().map_err(|e| e.to_string())?;
    let entry = connections.get(&conn_id).ok_or("Connection not found")?;
    entry.connection.log_collector.clear();
    Ok(())
}

#[tauri::command]
pub fn export_logs_csv(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<String, String> {
    let connections = state.connections.read().map_err(|e| e.to_string())?;
    let entry = connections.get(&conn_id).ok_or("Connection not found")?;
    Ok(entry.connection.log_collector.export_csv())
}

// ── Group Commands ──────────────────────────────────────────

#[tauri::command]
pub fn create_group(
    state: State<'_, AppState>,
    name: String,
) -> Result<NodeGroupDto, String> {
    let id = Uuid::new_v4().to_string();
    let group = NodeGroup {
        id: id.clone(),
        name: name.clone(),
        node_ids: vec![],
    };
    let mut groups = state.groups.write().map_err(|e| e.to_string())?;
    groups.push(group);
    Ok(NodeGroupDto { id, name, node_count: 0 })
}

#[tauri::command]
pub fn delete_group(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut groups = state.groups.write().map_err(|e| e.to_string())?;
    groups.retain(|g| g.id != id);
    Ok(())
}

#[tauri::command]
pub fn list_groups(
    state: State<'_, AppState>,
) -> Result<Vec<NodeGroupDto>, String> {
    let groups = state.groups.read().map_err(|e| e.to_string())?;
    Ok(groups.iter().map(|g| NodeGroupDto {
        id: g.id.clone(),
        name: g.name.clone(),
        node_count: g.node_ids.len(),
    }).collect())
}

#[tauri::command]
pub fn add_nodes_to_group(
    state: State<'_, AppState>,
    group_id: String,
    node_ids: Vec<String>,
) -> Result<(), String> {
    let mut groups = state.groups.write().map_err(|e| e.to_string())?;
    let group = groups.iter_mut().find(|g| g.id == group_id).ok_or("Group not found")?;
    for nid in node_ids {
        if !group.node_ids.contains(&nid) {
            group.node_ids.push(nid);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn remove_nodes_from_group(
    state: State<'_, AppState>,
    group_id: String,
    node_ids: Vec<String>,
) -> Result<(), String> {
    let mut groups = state.groups.write().map_err(|e| e.to_string())?;
    let group = groups.iter_mut().find(|g| g.id == group_id).ok_or("Group not found")?;
    group.node_ids.retain(|id| !node_ids.contains(id));
    Ok(())
}

// ── Project File Commands ──────────────────────────────────────────

#[tauri::command]
pub async fn save_project(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let connections = state.connections.read().map_err(|e| e.to_string())?;
    let groups = state.groups.read().map_err(|e| e.to_string())?;

    let mut project = ProjectFile::new_master();
    project.groups = groups.clone();

    for (_id, entry) in connections.iter() {
        let config = &entry.connection.config;
        project.connections.push(ConnectionProjectEntry {
            name: config.name.clone(),
            endpoint_url: config.endpoint_url.clone(),
            security_policy: config.security_policy.clone(),
            security_mode: config.security_mode.clone(),
            auth: config.auth.clone(),
            timeout_ms: config.timeout_ms,
            monitored_nodes: vec![], // TODO: populate from subscription/polling managers
        });
    }

    let json = project.to_json().map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn load_project(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let project = ProjectFile::from_json(&json).map_err(|e| e.to_string())?;

    let mut connections = state.connections.write().map_err(|e| e.to_string())?;
    connections.clear();

    for conn_entry in &project.connections {
        let id = Uuid::new_v4().to_string();
        let config = ConnectionConfig {
            id: id.clone(),
            name: conn_entry.name.clone(),
            endpoint_url: conn_entry.endpoint_url.clone(),
            security_policy: conn_entry.security_policy.clone(),
            security_mode: conn_entry.security_mode.clone(),
            auth: conn_entry.auth.clone(),
            timeout_ms: conn_entry.timeout_ms,
        };
        let connection = OpcUaConnection::new(config);
        connections.insert(id, ConnectionEntry { connection });
    }

    let mut groups = state.groups.write().map_err(|e| e.to_string())?;
    *groups = project.groups;

    Ok(())
}
```

- [ ] **Step 3: Update lib.rs with all commands**

```rust
mod commands;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Connection commands
            commands::create_connection,
            commands::connect,
            commands::disconnect,
            commands::delete_connection,
            commands::list_connections,
            commands::get_endpoints,
            // Log commands
            commands::get_communication_logs,
            commands::clear_communication_logs,
            commands::export_logs_csv,
            // Group commands
            commands::create_group,
            commands::delete_group,
            commands::list_groups,
            commands::add_nodes_to_group,
            commands::remove_nodes_from_group,
            // Project file commands
            commands::save_project,
            commands::load_project,
        ])
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check --workspace`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add crates/opcuamaster-app/src/
git commit -m "feat: add Tauri commands for connection CRUD, logs, groups, and project files"
```

---

## Task 8: Integrate async-opcua — Real Connection, Browse, Subscribe, Poll

**Files:**
- Modify: `crates/opcuasim-core/src/client.rs`
- Modify: `crates/opcuasim-core/src/browse.rs`
- Modify: `crates/opcuasim-core/src/subscription.rs`
- Modify: `crates/opcuasim-core/src/polling.rs`

This task replaces the TODO stubs with actual `async-opcua` calls. The exact API depends on `async-opcua-client v0.18`, so the implementer should:

- [ ] **Step 1: Read async-opcua documentation and examples**

Run: `cargo doc -p async-opcua-client --open` or check the crate's GitHub README for the client API.

Key APIs to use:
- `ClientBuilder::new()` → build a `Client`
- `client.connect_to_endpoint(endpoint_url, security_policy, ...)` → `Session`
- `session.browse(...)` → browse results
- `session.read(...)` → read values
- `session.create_subscription(...)` → subscription ID
- `session.create_monitored_items(subscription_id, ...)` → monitored items
- Data change callback on the subscription

- [ ] **Step 2: Update client.rs — implement connect() with async-opcua**

Replace the TODO in `connect()` with actual session creation using `async-opcua-client`. Store the session handle in the `OpcUaConnection` struct (add a field `session: Arc<RwLock<Option<Session>>>`).

Handle authentication variants (Anonymous, UserPassword, Certificate) based on `self.config.auth`.

Handle security policy/mode mapping from the string config to the async-opcua enum types.

- [ ] **Step 3: Update browse.rs — implement browse_node() and read_node_attributes()**

Use the session's `browse()` method to enumerate child nodes. Map the OPC UA `ReferenceDescription` results to `BrowseResultItem`.

Use the session's `read()` method to fetch node attributes (DisplayName, Description, DataType, Value, etc.). Map to `NodeAttributes`.

- [ ] **Step 4: Update subscription.rs — implement actual OPC UA subscriptions**

When `add_nodes()` is called, use the session to:
1. `create_subscription(interval, ...)` if no subscription exists yet
2. `create_monitored_items(subscription_id, items)` for each node

Set up the data change callback to call `apply_data_changes()` and emit a Tauri event.

- [ ] **Step 5: Update polling.rs — implement actual read in the polling loop**

In the `tokio::spawn` loop, use the session's `read()` method to read the current value of the node, then update `monitored_items` with the result.

- [ ] **Step 6: Add browse and monitored node commands to Tauri**

Add to `commands.rs`:
```rust
#[tauri::command]
pub async fn browse_root(state: State<'_, AppState>, conn_id: String) -> Result<Vec<BrowseResultDto>, String> { ... }

#[tauri::command]
pub async fn browse_node(state: State<'_, AppState>, conn_id: String, node_id: String) -> Result<Vec<BrowseResultDto>, String> { ... }

#[tauri::command]
pub async fn read_node_attributes(state: State<'_, AppState>, conn_id: String, node_id: String) -> Result<NodeAttributesDto, String> { ... }

#[tauri::command]
pub async fn add_monitored_nodes(state: State<'_, AppState>, conn_id: String, nodes: Vec<AddNodeRequest>, access_mode: String, interval_ms: f64) -> Result<(), String> { ... }

#[tauri::command]
pub async fn remove_monitored_nodes(state: State<'_, AppState>, conn_id: String, node_ids: Vec<String>) -> Result<(), String> { ... }

#[tauri::command]
pub async fn get_monitored_data(state: State<'_, AppState>, conn_id: String, since_seq: u64) -> Result<Vec<MonitoredNodeDto>, String> { ... }
```

Register all new commands in `lib.rs`.

- [ ] **Step 7: Verify with a test OPC UA server**

Run an OPC UA test server (e.g., `async-opcua`'s built-in simple-server example or Prosys Simulation Server).

Run: `cd crates/opcuamaster-app && cargo tauri dev`

Manual test:
1. Create a connection to the test server
2. Connect
3. Browse root nodes
4. Add a node to monitoring (subscription mode)
5. Verify data changes appear in logs

- [ ] **Step 8: Commit**

```bash
git add crates/
git commit -m "feat: integrate async-opcua for real OPC UA connection, browse, subscribe, and poll"
```

---

## Task 9: Frontend — Toolbar Component

**Files:**
- Create: `master-frontend/src/components/Toolbar.vue`
- Modify: `master-frontend/src/App.vue` (replace placeholder)

- [ ] **Step 1: Create Toolbar.vue**

Implement toolbar with buttons for:
- New Connection (opens a dialog to input: name, endpoint URL, security policy, security mode, auth type, username/password)
- Connect / Disconnect (based on selected connection state)
- Delete Connection
- Browse Nodes (opens BrowsePanel)
- Save Project / Open Project (.opcuaproj via tauri-plugin-dialog)
- Export Logs

Follow the exact same styling pattern as ModbusSim's Toolbar.vue:
- 42px height, `#1e1e2e` background
- Buttons with `#313244` background, `#cdd6f4` text, 4px border-radius
- Separator dividers between button groups
- Hover state: `#45475a` background

Wire each button to the corresponding Tauri `invoke()` call.

- [ ] **Step 2: Update App.vue to use Toolbar**

Replace the toolbar placeholder `<div>` with `<Toolbar />` import.

- [ ] **Step 3: Verify toolbar renders and buttons invoke commands**

Run: `cd crates/opcuamaster-app && cargo tauri dev`
Expected: Toolbar visible, "New Connection" dialog works, connection appears in tree after creation

- [ ] **Step 4: Commit**

```bash
git add master-frontend/src/components/Toolbar.vue master-frontend/src/App.vue
git commit -m "feat: add Toolbar component with connection CRUD and project file support"
```

---

## Task 10: Frontend — ConnectionTree Component

**Files:**
- Create: `master-frontend/src/components/ConnectionTree.vue`
- Modify: `master-frontend/src/App.vue` (replace placeholder)

- [ ] **Step 1: Create ConnectionTree.vue**

Implement a tree with two view modes (toggle at top):

**Address Space View:**
- List connections (show name, endpoint, state indicator)
- Under each connection, show browse results (lazy-loaded on expand)
- Right-click context menu: "Add to monitoring", "Add to group"

**Group View:**
- List groups (show name, node count)
- Under each group, flat list of node IDs
- Right-click: "Remove from group"

Follow ConnectionTree.vue pattern from ModbusSim:
- `#181825` background, items with hover `#313244`
- Selected item: `#313244` background with `#89b4fa` left border
- Expand/collapse chevrons
- State indicator dots (green=connected, gray=disconnected, yellow=reconnecting)

Wire:
- `@connection-select` emit when a connection is clicked
- `@node-select` emit when monitoring nodes are clicked
- Refresh when `treeRefreshKey` changes

- [ ] **Step 2: Update App.vue**

Replace tree placeholder with `<ConnectionTree />`, wire events.

- [ ] **Step 3: Verify**

Run: `cd crates/opcuamaster-app && cargo tauri dev`
Expected: Connections shown in tree, expandable, state indicators work

- [ ] **Step 4: Commit**

```bash
git add master-frontend/src/components/ConnectionTree.vue master-frontend/src/App.vue
git commit -m "feat: add ConnectionTree with address space and group views"
```

---

## Task 11: Frontend — BrowsePanel Component

**Files:**
- Create: `master-frontend/src/components/BrowsePanel.vue`
- Modify: `master-frontend/src/App.vue` (add modal trigger)

- [ ] **Step 1: Create BrowsePanel.vue**

Modal dialog with a lazy-loading tree:
- Starts from Objects root node
- Click expand arrow → calls `invoke('browse_node', { connId, nodeId })`
- Shows: NodeId, DisplayName, NodeClass, DataType
- Checkboxes for multi-select
- "Add to Monitoring" button at bottom (calls `add_monitored_nodes`)
- Optionally choose subscription vs polling mode and interval

Style: modal overlay with `#1e1e2e` dialog, same as AppDialog pattern but larger (600x500).

- [ ] **Step 2: Wire BrowsePanel in App.vue**

Add a `showBrowsePanel` ref, toggled by Toolbar's "Browse Nodes" button. Render `<BrowsePanel v-if="showBrowsePanel" />`.

- [ ] **Step 3: Verify**

Connect to a test server, click Browse, expand nodes, select and add to monitoring.

- [ ] **Step 4: Commit**

```bash
git add master-frontend/src/components/BrowsePanel.vue master-frontend/src/App.vue
git commit -m "feat: add BrowsePanel modal for OPC UA node discovery"
```

---

## Task 12: Frontend — DataTable Component

**Files:**
- Create: `master-frontend/src/components/DataTable.vue`
- Modify: `master-frontend/src/App.vue` (replace placeholder)

- [ ] **Step 1: Create DataTable.vue**

Virtual-scrolled table showing monitored nodes:
- Columns: NodeId, DisplayName, DataType, Value, Quality, Timestamp, AccessMode
- Uses `@tanstack/vue-virtual` for virtual scrolling
- Incremental poll: call `invoke('get_monitored_data', { connId, sinceSeq })` on a 2-second timer
- Row selection: click to select, Ctrl+click for multi-select
- Right-click context menu: Write Value, Switch Mode, Remove, Add to Group
- Emit `@node-select` with selected `MonitoredNodeInfo[]`

Follow DataTable.vue pattern from ModbusSim:
- Header row: `#1e1e2e` with `#a6adc8` text
- Data rows: alternating `#11111b` / `#181825`
- Selected row: `#313244`
- Value column: monospace font

- [ ] **Step 2: Update App.vue**

Replace content placeholder with `<DataTable @node-select="handleNodeSelect" />`.

- [ ] **Step 3: Verify**

Add monitored nodes, verify they appear in table with live value updates.

- [ ] **Step 4: Commit**

```bash
git add master-frontend/src/components/DataTable.vue master-frontend/src/App.vue
git commit -m "feat: add DataTable with virtual scrolling and incremental data updates"
```

---

## Task 13: Frontend — ValuePanel Component

**Files:**
- Create: `master-frontend/src/components/ValuePanel.vue`
- Modify: `master-frontend/src/App.vue` (replace placeholder)

- [ ] **Step 1: Create ValuePanel.vue**

Right-side panel showing selected node details:
- Node ID, Display Name, Description, Data Type, Access Level
- Current Value display (formatted by data type)
- Quality indicator
- Timestamp
- Manual Read button (calls `read_node_attributes`)
- Write Value input + Write button
- Access mode toggle (Subscription ↔ Polling) with interval input

Follow ValuePanel.vue pattern from ModbusSim:
- `#181825` background
- Section headers with `#a6adc8` text, 11px uppercase
- Values in `#cdd6f4`, monospace for numeric values
- Buttons matching toolbar style

- [ ] **Step 2: Update App.vue**

Replace panel placeholder with `<ValuePanel />`.

- [ ] **Step 3: Verify**

Select a node in DataTable, verify attributes show in ValuePanel, test read/write.

- [ ] **Step 4: Commit**

```bash
git add master-frontend/src/components/ValuePanel.vue master-frontend/src/App.vue
git commit -m "feat: add ValuePanel for node details, read/write, and access mode control"
```

---

## Task 14: Frontend — LogPanel Component

**Files:**
- Create: `master-frontend/src/components/LogPanel.vue`
- Modify: `master-frontend/src/App.vue` (replace placeholder)

- [ ] **Step 1: Create LogPanel.vue**

Collapsible log panel at the bottom:
- Header bar (32px): "Communication Log" label, connection dropdown, search, direction filter, service filter, CSV export, clear
- Expanded area (200px): scrollable log entries
- Columns: Timestamp, Direction (Request=green, Response=blue), Service, Detail, Status
- Auto-refresh every 2 seconds when expanded
- Uses `useLogPanel` and `useLogFilter` from shared-frontend

Follow LogPanel.vue pattern from ModbusSim exactly.

- [ ] **Step 2: Update App.vue**

Replace log placeholder with `<LogPanel :expanded="logExpanded" @toggle="toggleLog" />`.

- [ ] **Step 3: Verify**

Connect to server, perform operations, verify logs appear with correct direction and service type.

- [ ] **Step 4: Commit**

```bash
git add master-frontend/src/components/LogPanel.vue master-frontend/src/App.vue
git commit -m "feat: add LogPanel with filtering, search, and CSV export"
```

---

## Task 15: End-to-End Testing and Polish

**Files:**
- Various fixes across all files

- [ ] **Step 1: Start a test OPC UA server**

Use async-opcua's simple-server example or download Prosys OPC UA Simulation Server.

- [ ] **Step 2: Full workflow test**

1. Launch app: `cd crates/opcuamaster-app && cargo tauri dev`
2. Create connection to test server
3. Connect
4. Browse nodes → add 5-10 nodes to monitoring (mix subscription and polling)
5. Verify data updates in DataTable
6. Create a group, add nodes to it, switch to group view
7. Select a node, read/write from ValuePanel
8. Check logs in LogPanel, test filters, export CSV
9. Save project as .opcuaproj
10. Delete all connections, load project, verify restored
11. Disconnect, verify reconnect indicator shows

- [ ] **Step 3: Fix any issues found during testing**

Address UI glitches, data flow bugs, error handling gaps.

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "fix: end-to-end testing fixes and polish"
```

---

## Notes for Implementer

- **Port 5178**: The dev server port is set to 5178 to avoid conflicting with ModbusSim (5174/5175) and IEC104 (5176/5177).
- **async-opcua API**: The exact async-opcua v0.18 API may differ from what's shown. Consult the crate docs. The key patterns (ClientBuilder → Session → browse/read/subscribe) should be stable.
- **Tauri State + async**: When holding `state.connections.read()` across an `.await`, you may need to restructure to avoid holding the lock across await points. Clone what you need before the async call.
- **shared-frontend**: This project creates its own shared-frontend rather than sharing with ModbusSim, since they're in different repos. The code is copied from the ModbusSim pattern.
