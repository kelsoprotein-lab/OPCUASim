<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ServerNodeInfo, ServerFolderInfo, ServerStatus, AddressSpace } from './types'
import Toolbar from './components/Toolbar.vue'
import AddressSpaceTree from './components/AddressSpaceTree.vue'
import NodeTable from './components/NodeTable.vue'
import PropertyEditor from './components/PropertyEditor.vue'
import StatusBar from './components/StatusBar.vue'

const folders = ref<ServerFolderInfo[]>([])
const nodes = ref<ServerNodeInfo[]>([])
const status = ref<ServerStatus>({ state: 'Stopped', node_count: 0, folder_count: 0 })
const selectedNodeId = ref<string | null>(null)

const selectedNode = ref<ServerNodeInfo | null>(null)

let pollTimer: ReturnType<typeof setInterval> | null = null
let simSeq = 0

async function refreshAddressSpace() {
  try {
    const data = await invoke<AddressSpace>('get_address_space')
    folders.value = data.folders
    nodes.value = data.nodes
  } catch (e) {
    console.error('Failed to get address space:', e)
  }
}

async function refreshStatus() {
  try {
    status.value = await invoke<ServerStatus>('get_server_status')
  } catch (e) {
    console.error('Failed to get status:', e)
  }
}

interface SimulationData {
  values: { node_id: string; value: string }[]
  seq: number
}

async function pollSimulationData() {
  if (status.value.state !== 'Running') return
  try {
    const data = await invoke<SimulationData>('get_simulation_data', { sinceSeq: simSeq })
    if (data.values.length > 0) {
      const valueMap = new Map(data.values.map(v => [v.node_id, v.value]))
      nodes.value = nodes.value.map(n => {
        const newVal = valueMap.get(n.node_id)
        return newVal !== undefined ? { ...n, current_value: newVal } : n
      })
      // Update selected node if it changed
      if (selectedNodeId.value) {
        selectedNode.value = nodes.value.find(n => n.node_id === selectedNodeId.value) ?? null
      }
    }
    simSeq = data.seq
  } catch (e) {
    console.error('Failed to poll simulation data:', e)
  }
}

function onSelectNode(nodeId: string | null) {
  selectedNodeId.value = nodeId
  selectedNode.value = nodeId ? nodes.value.find(n => n.node_id === nodeId) ?? null : null
}

async function onStartServer() {
  try {
    await invoke('start_server')
    await refreshStatus()
  } catch (e) {
    alert('Start failed: ' + e)
  }
}

async function onStopServer() {
  try {
    await invoke('stop_server')
    await refreshStatus()
  } catch (e) {
    alert('Stop failed: ' + e)
  }
}

async function onAddFolder(displayName: string, parentId: string) {
  const nodeId = `Folder_${Date.now()}`
  try {
    await invoke('add_folder', { nodeId, displayName, parentId: parentId || 'i=85' })
    await refreshAddressSpace()
  } catch (e) {
    alert('Add folder failed: ' + e)
  }
}

async function onAddNode(params: {
  displayName: string
  parentId: string
  dataType: string
  writable: boolean
  simulation: Record<string, unknown>
}) {
  const nodeId = `Node_${Date.now()}`
  try {
    await invoke('add_node', {
      params: {
        node_id: nodeId,
        display_name: params.displayName,
        parent_id: params.parentId || 'i=85',
        data_type: params.dataType,
        writable: params.writable,
        simulation: params.simulation,
      }
    })
    await refreshAddressSpace()
  } catch (e) {
    alert('Add node failed: ' + e)
  }
}

async function onRemoveNode(nodeId: string) {
  try {
    await invoke('remove_node', { nodeId })
    if (selectedNodeId.value === nodeId) {
      selectedNodeId.value = null
      selectedNode.value = null
    }
    await refreshAddressSpace()
  } catch (e) {
    alert('Remove failed: ' + e)
  }
}

onMounted(() => {
  refreshAddressSpace()
  refreshStatus()
  pollTimer = setInterval(() => {
    refreshStatus()
    pollSimulationData()
  }, 1000)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
})
</script>

<template>
  <div class="app">
    <Toolbar
      :status="status"
      @start="onStartServer"
      @stop="onStopServer"
      @add-folder="onAddFolder"
      @add-node="onAddNode"
    />
    <div class="main-area">
      <AddressSpaceTree
        :folders="folders"
        :nodes="nodes"
        :selected-node-id="selectedNodeId"
        @select="onSelectNode"
        @remove="onRemoveNode"
      />
      <NodeTable
        :nodes="nodes"
        :selected-node-id="selectedNodeId"
        @select="onSelectNode"
      />
      <PropertyEditor
        :node="selectedNode"
      />
    </div>
    <StatusBar :status="status" :node-count="nodes.length" :folder-count="folders.length" />
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: #1e1e2e;
  color: #cdd6f4;
  overflow: hidden;
}

.app {
  display: grid;
  grid-template-rows: 42px 1fr 28px;
  height: 100vh;
  width: 100vw;
}

.main-area {
  display: grid;
  grid-template-columns: 240px 1fr 280px;
  overflow: hidden;
  border-top: 1px solid #313244;
  border-bottom: 1px solid #313244;
}
</style>
