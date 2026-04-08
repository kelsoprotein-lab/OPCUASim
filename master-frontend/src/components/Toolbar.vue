<script setup lang="ts">
import { inject, computed, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { save, open } from '@tauri-apps/plugin-dialog'
import { dialogKey } from '../composables/useDialog'

const emit = defineEmits<{
  browse: []
}>()

const selectedConnectionId = inject<Ref<string | null>>('selectedConnectionId')!
const selectedConnectionState = inject<Ref<string>>('selectedConnectionState')!
const refreshTree = inject<() => void>('refreshTree')!
const refreshData = inject<() => void>('refreshData')!
const dialog = inject(dialogKey)!

const hasConnection = computed(() => selectedConnectionId.value !== null)
const isConnected = computed(() => selectedConnectionState.value === 'Connected')
const isDisconnected = computed(() => selectedConnectionState.value === 'Disconnected')

async function newConnection() {
  const name = await dialog.showPrompt('New Connection', 'Connection name:', 'Local Server')
  if (!name) return
  const url = await dialog.showPrompt('Endpoint URL', 'OPC UA endpoint:', 'opc.tcp://localhost:4840')
  if (!url) return

  try {
    await invoke('create_connection', {
      request: { name, endpoint_url: url }
    })
    refreshTree()
  } catch (e) {
    await dialog.showAlert('Error', String(e))
  }
}

async function connectSelected() {
  if (!selectedConnectionId.value) return
  try {
    await invoke('connect', { id: selectedConnectionId.value })
    refreshTree()
  } catch (e) {
    await dialog.showAlert('Error', String(e))
  }
}

async function disconnectSelected() {
  if (!selectedConnectionId.value) return
  try {
    await invoke('disconnect', { id: selectedConnectionId.value })
    refreshTree()
  } catch (e) {
    await dialog.showAlert('Error', String(e))
  }
}

async function deleteSelected() {
  if (!selectedConnectionId.value) return
  const confirmed = await dialog.showConfirm('Delete Connection', 'Are you sure you want to delete this connection?')
  if (!confirmed) return
  try {
    await invoke('delete_connection', { id: selectedConnectionId.value })
    selectedConnectionId.value = null
    refreshTree()
  } catch (e) {
    await dialog.showAlert('Error', String(e))
  }
}

async function browseNodes() {
  emit('browse')
}

async function saveProject() {
  const path = await save({
    filters: [{ name: 'OPC UA Project', extensions: ['opcuaproj'] }],
  })
  if (!path) return
  try {
    await invoke('save_project', { path })
    await dialog.showAlert('Saved', 'Project saved successfully.')
  } catch (e) {
    await dialog.showAlert('Error', String(e))
  }
}

async function openProject() {
  const path = await open({
    filters: [{ name: 'OPC UA Project', extensions: ['opcuaproj'] }],
  })
  if (!path) return
  try {
    await invoke('load_project', { path })
    refreshTree()
    refreshData()
  } catch (e) {
    await dialog.showAlert('Error', String(e))
  }
}

async function exportLogs() {
  if (!selectedConnectionId.value) return
  try {
    const csv = await invoke<string>('export_logs_csv', { connId: selectedConnectionId.value })
    const blob = new Blob([csv], { type: 'text/csv' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `opcua-logs-${selectedConnectionId.value}.csv`
    a.click()
    URL.revokeObjectURL(url)
  } catch (e) {
    await dialog.showAlert('Error', String(e))
  }
}
</script>

<template>
  <div class="toolbar">
    <button class="tb-btn" @click="newConnection">New Connection</button>
    <button class="tb-btn" :disabled="!hasConnection || isConnected" @click="connectSelected">Connect</button>
    <button class="tb-btn" :disabled="!hasConnection || isDisconnected" @click="disconnectSelected">Disconnect</button>
    <button class="tb-btn" :disabled="!hasConnection" @click="deleteSelected">Delete</button>

    <div class="separator" />

    <button class="tb-btn" :disabled="!isConnected" @click="browseNodes">Browse Nodes</button>

    <div class="separator" />

    <button class="tb-btn" @click="saveProject">Save</button>
    <button class="tb-btn" @click="openProject">Open</button>

    <div class="separator" />

    <button class="tb-btn" :disabled="!hasConnection" @click="exportLogs">Export Logs</button>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0 8px;
  height: 100%;
  background: #1e1e2e;
}

.tb-btn {
  background: #313244;
  color: #cdd6f4;
  border: none;
  border-radius: 4px;
  padding: 4px 12px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}

.tb-btn:hover:not(:disabled) {
  background: #45475a;
}

.tb-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.separator {
  width: 1px;
  height: 20px;
  background: #313244;
  margin: 0 6px;
}
</style>
