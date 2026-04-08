<script setup lang="ts">
import { ref, inject, watch, onMounted, onUnmounted, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { MonitoredNodeInfo } from '../types'

const emit = defineEmits<{
  'node-select': [nodes: MonitoredNodeInfo[]]
}>()

const selectedConnectionId = inject<Ref<string | null>>('selectedConnectionId')!
const dataRefreshKey = inject<Ref<number>>('dataRefreshKey')!

const nodes = ref<MonitoredNodeInfo[]>([])
const selectedNodeIds = ref<Set<string>>(new Set())
let pollTimer: ReturnType<typeof setInterval> | null = null

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
    // get_monitored_data not registered yet — expected until Task 8
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
    // Toggle selection
    if (selectedNodeIds.value.has(node.node_id)) {
      selectedNodeIds.value.delete(node.node_id)
    } else {
      selectedNodeIds.value.add(node.node_id)
    }
  } else {
    // Single select
    selectedNodeIds.value.clear()
    selectedNodeIds.value.add(node.node_id)
  }
  // Force reactivity
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
    <div class="table-header">
      <div class="th" style="width: 200px">NodeId</div>
      <div class="th" style="width: 150px">Name</div>
      <div class="th" style="width: 80px">Type</div>
      <div class="th" style="flex: 1">Value</div>
      <div class="th" style="width: 60px">Quality</div>
      <div class="th" style="width: 160px">Timestamp</div>
      <div class="th" style="width: 80px">Mode</div>
    </div>

    <div class="table-body">
      <div v-if="nodes.length === 0" class="empty-state">
        No monitored nodes. Use "Browse Nodes" to add nodes.
      </div>
      <div
        v-for="(node, i) in nodes"
        :key="node.node_id"
        :class="['table-row', { selected: isSelected(node), alt: i % 2 === 1 }]"
        @click="selectNode(node, $event)"
      >
        <div class="td mono" style="width: 200px">{{ node.node_id }}</div>
        <div class="td" style="width: 150px">{{ node.display_name }}</div>
        <div class="td" style="width: 80px">{{ node.data_type }}</div>
        <div class="td mono" style="flex: 1">{{ node.value || '—' }}</div>
        <div class="td" style="width: 60px">
          <span :style="{ color: qualityColor(node.quality) }">{{ node.quality || '—' }}</span>
        </div>
        <div class="td mono" style="width: 160px">{{ node.timestamp || '—' }}</div>
        <div class="td" style="width: 80px">
          <span class="mode-badge">{{ node.access_mode }}</span>
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

.table-header {
  display: flex;
  background: #1e1e2e;
  border-bottom: 1px solid #313244;
  flex-shrink: 0;
}

.th {
  padding: 6px 8px;
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
  border-bottom: 1px solid #181825;
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
  padding: 4px 8px;
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

.mode-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
  background: #313244;
  color: #a6adc8;
}
</style>
