<script setup lang="ts">
import { ref, provide, onMounted, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { AppDialog } from 'shared-frontend'
import { showAlert, showConfirm, showPrompt, dialogKey } from './composables/useDialog'
import type { ConnectionStateEvent, MonitoredNodeInfo } from './types'
import Toolbar from './components/Toolbar.vue'
import ConnectionTree from './components/ConnectionTree.vue'
import LogPanel from './components/LogPanel.vue'
import BrowsePanel from './components/BrowsePanel.vue'
import DataTable from './components/DataTable.vue'
import ValuePanel from './components/ValuePanel.vue'

// Shared state
const selectedConnectionId = ref<string | null>(null)
const selectedConnectionState = ref<string>('Disconnected')
const selectedNodes = ref<MonitoredNodeInfo[]>([])
const logExpanded = ref(false)
const showBrowsePanel = ref(false)

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
      <Toolbar @browse="showBrowsePanel = true" />
    </header>

    <aside class="tree-area">
      <ConnectionTree
        @connection-select="handleConnectionSelect"
      />
    </aside>
    <main class="content-area">
      <DataTable @node-select="handleNodeSelect" />
    </main>
    <aside class="panel-area">
      <ValuePanel />
    </aside>

    <footer class="log-area">
      <LogPanel :expanded="logExpanded" @toggle="toggleLog" />
    </footer>
    <AppDialog />
    <BrowsePanel :visible="showBrowsePanel" @close="showBrowsePanel = false" />
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
