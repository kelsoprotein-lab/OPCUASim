<script setup lang="ts">
import { ref, inject, watch, computed, onMounted, onUnmounted, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { MonitoredNodeInfo } from '../types'

const emit = defineEmits<{
  'node-select': [nodes: MonitoredNodeInfo[]]
}>()

const selectedConnectionId = inject<Ref<string | null>>('selectedConnectionId')!
const dataRefreshKey = inject<Ref<number>>('dataRefreshKey')!

const nodes = ref<MonitoredNodeInfo[]>([])
const selectedNodeIds = ref<Set<string>>(new Set())
const searchQuery = ref('')
let pollTimer: ReturnType<typeof setInterval> | null = null

// Extract short identifier from NodeId (e.g. "ns=3;s=OPCUA1-xxx" → "OPCUA1-xxx")
function shortNodeId(nodeId: string): string {
  const parts = nodeId.split(';')
  const last = parts[parts.length - 1]
  // Remove "s=", "i=", "g=", "b=" prefix
  return last.replace(/^[sigb]=/, '')
}

// Filter nodes by search query
const filteredNodes = computed(() => {
  if (!searchQuery.value) return nodes.value
  const q = searchQuery.value.toLowerCase()
  return nodes.value.filter((n) =>
    n.node_id.toLowerCase().includes(q) ||
    n.display_name.toLowerCase().includes(q) ||
    (n.value && n.value.toLowerCase().includes(q))
  )
})

async function loadData() {
  if (!selectedConnectionId.value) {
    nodes.value = []
    return
  }
  try {
    const data = await invoke<{ nodes: MonitoredNodeInfo[], seq: number }>('get_monitored_data', {
      connId: selectedConnectionId.value,
      sinceSeq: 0,
    }).catch(() => ({ nodes: [], seq: 0 }))
    nodes.value = data.nodes
  } catch {
    // silent
  }
}

function startPolling() {
  stopPolling()
  if (selectedConnectionId.value) {
    pollTimer = setInterval(loadData, 2000)
  }
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

watch(selectedConnectionId, () => {
  loadData()
  startPolling()
})

watch(dataRefreshKey, loadData)

onMounted(() => {
  loadData()
  startPolling()
})

onUnmounted(stopPolling)

function selectNode(node: MonitoredNodeInfo, event: MouseEvent) {
  if (event.ctrlKey || event.metaKey) {
    if (selectedNodeIds.value.has(node.node_id)) {
      selectedNodeIds.value.delete(node.node_id)
    } else {
      selectedNodeIds.value.add(node.node_id)
    }
  } else {
    selectedNodeIds.value.clear()
    selectedNodeIds.value.add(node.node_id)
  }
  selectedNodeIds.value = new Set(selectedNodeIds.value)
  const selected = nodes.value.filter((n) => selectedNodeIds.value.has(n.node_id))
  emit('node-select', selected)
}

function isSelected(node: MonitoredNodeInfo): boolean {
  return selectedNodeIds.value.has(node.node_id)
}

function qualityColor(quality?: string): string {
  if (!quality) return '#585b70'
  if (quality === 'Good') return '#a6e3a1'
  if (quality.startsWith('Bad')) return '#f38ba8'
  return '#f9e2af'
}
</script>

<template>
  <div class="data-table-container">
    <!-- Search bar -->
    <div class="search-bar">
      <input
        v-model="searchQuery"
        class="search-input"
        placeholder="Search NodeId, Name, Value..."
      />
      <span class="node-count">{{ filteredNodes.length }} / {{ nodes.length }} nodes</span>
    </div>

    <!-- Table header -->
    <div class="table-header">
      <div class="th" style="width: 180px">NodeId</div>
      <div class="th" style="flex: 1; min-width: 200px">Name</div>
      <div class="th" style="width: 100px">Value</div>
      <div class="th" style="width: 55px">Quality</div>
      <div class="th" style="width: 140px">Timestamp</div>
      <div class="th" style="width: 70px">Mode</div>
    </div>

    <!-- Table body -->
    <div class="table-body">
      <div v-if="nodes.length === 0" class="empty-state">
        No monitored nodes. Use "Browse Nodes" to add nodes.
      </div>
      <div
        v-for="(node, i) in filteredNodes"
        :key="node.node_id"
        :class="['table-row', { selected: isSelected(node), alt: i % 2 === 1 }]"
        @click="selectNode(node, $event)"
      >
        <div class="td mono" style="width: 180px" :title="node.node_id">
          {{ shortNodeId(node.node_id) }}
        </div>
        <div class="td" style="flex: 1; min-width: 200px" :title="node.display_name">
          {{ node.display_name }}
        </div>
        <div class="td mono value-cell" style="width: 100px" :title="node.value || ''">
          {{ node.value || '—' }}
        </div>
        <div class="td" style="width: 55px">
          <span :style="{ color: qualityColor(node.quality) }">{{ node.quality || '—' }}</span>
        </div>
        <div class="td mono" style="width: 140px" :title="node.timestamp || ''">
          {{ node.timestamp ? node.timestamp.substring(11, 19) : '—' }}
        </div>
        <div class="td" style="width: 70px">
          <span class="mode-badge">{{ node.access_mode === 'Subscription' ? 'Sub' : 'Poll' }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.data-table-container {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.search-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  background: #1e1e2e;
  border-bottom: 1px solid #313244;
  flex-shrink: 0;
}

.search-input {
  flex: 1;
  padding: 4px 10px;
  background: #11111b;
  border: 1px solid #313244;
  border-radius: 4px;
  color: #cdd6f4;
  font-size: 12px;
  outline: none;
}

.search-input:focus {
  border-color: #89b4fa;
}

.node-count {
  font-size: 11px;
  color: #585b70;
  white-space: nowrap;
}

.table-header {
  display: flex;
  background: #181825;
  border-bottom: 1px solid #313244;
  flex-shrink: 0;
}

.th {
  padding: 5px 8px;
  font-size: 11px;
  font-weight: 500;
  color: #a6adc8;
  text-transform: uppercase;
  white-space: nowrap;
}

.table-body {
  flex: 1;
  overflow-y: auto;
}

.empty-state {
  padding: 24px;
  text-align: center;
  color: #585b70;
  font-size: 13px;
}

.table-row {
  display: flex;
  cursor: pointer;
  border-bottom: 1px solid rgba(49, 50, 68, 0.5);
}

.table-row:hover {
  background: rgba(137, 180, 250, 0.05);
}

.table-row.selected {
  background: #313244;
}

.table-row.alt {
  background: #181825;
}

.table-row.alt.selected {
  background: #313244;
}

.td {
  padding: 3px 8px;
  font-size: 12px;
  color: #cdd6f4;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mono {
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 11px;
}

.value-cell {
  color: #89b4fa;
  font-weight: 500;
}

.mode-badge {
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 3px;
  background: #313244;
  color: #a6adc8;
}
</style>
