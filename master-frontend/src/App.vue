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
provide('handleConnectionSelect', handleConnectionSelect)

function handleNodeSelect(nodes: MonitoredNodeInfo[]) {
  selectedNodes.value = nodes
}
provide('handleNodeSelect', handleNodeSelect)

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
