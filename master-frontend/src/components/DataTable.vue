<script setup lang="ts">
import { ref, inject, watch, computed, onMounted, onUnmounted, type Ref } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { invoke } from '@tauri-apps/api/core'
import type { MonitoredNodeInfo } from '../types'

const emit = defineEmits<{
  'node-select': [nodes: MonitoredNodeInfo[]]
}>()

const selectedConnectionId = inject<Ref<string | null>>('selectedConnectionId')!
const dataRefreshKey = inject<Ref<number>>('dataRefreshKey')!

const allNodes = ref<Map<string, MonitoredNodeInfo>>(new Map())
const selectedNodeIds = ref<Set<string>>(new Set())
const searchQuery = ref('')
const lastSeq = ref(0)
let pollTimer: ReturnType<typeof setInterval> | null = null
const tableBodyRef = ref<HTMLElement | null>(null)

function shortNodeId(nodeId: string): string {
  const parts = nodeId.split(';')
  const last = parts[parts.length - 1]
  return last.replace(/^[sigb]=/, '')
}

const nodeList = computed(() => Array.from(allNodes.value.values()))

const filteredNodes = computed(() => {
  const list = nodeList.value
  if (!searchQuery.value) return list
  const q = searchQuery.value.toLowerCase()
  return list.filter((n) =>
    n.node_id.toLowerCase().includes(q) ||
    n.display_name.toLowerCase().includes(q) ||
    (n.value && n.value.toLowerCase().includes(q))
  )
})

// Virtual scrolling
const virtualizer = useVirtualizer({
  get count() { return filteredNodes.value.length },
  getScrollElement: () => tableBodyRef.value,
  estimateSize: () => 24,
  overscan: 10,
})

async function loadData() {
  if (!selectedConnectionId.value) {
    allNodes.value = new Map()
    lastSeq.value = 0
    return
  }
  try {
    const data = await invoke<{ nodes: MonitoredNodeInfo[], seq: number }>('get_monitored_data', {
      connId: selectedConnectionId.value,
      sinceSeq: lastSeq.value,
    }).catch(() => ({ nodes: [], seq: 0 }))

    if (lastSeq.value === 0) {
      // Full load
      const map = new Map<string, MonitoredNodeInfo>()
      for (const n of data.nodes) map.set(n.node_id, n)
      allNodes.value = map
    } else {
      // Incremental merge
      if (data.nodes.length > 0) {
        const map = new Map(allNodes.value)
        for (const n of data.nodes) map.set(n.node_id, n)
        allNodes.value = map
      }
    }
    if (data.seq > 0) lastSeq.value = data.seq
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
  lastSeq.value = 0
  loadData()
  startPolling()
})

watch(dataRefreshKey, () => {
  lastSeq.value = 0
  loadData()
})

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
  const selected = nodeList.value.filter((n) => selectedNodeIds.value.has(n.node_id))
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
    <div class="search-bar">
      <input v-model="searchQuery" class="search-input" placeholder="Search NodeId, Name, Value..." />
      <span class="node-count">{{ filteredNodes.length }} / {{ nodeList.length }} nodes</span>
    </div>

    <div class="table-header">
      <div class="th col-id">NodeId</div>
      <div class="th col-name">Name</div>
      <div class="th col-value">Value</div>
      <div class="th col-quality">Quality</div>
      <div class="th col-time">Time</div>
      <div class="th col-mode">Mode</div>
    </div>

    <div ref="tableBodyRef" class="table-body">
      <div v-if="nodeList.length === 0" class="empty-state">
        No monitored nodes. Use "Browse Nodes" to add nodes.
      </div>
      <div v-else :style="{ height: `${virtualizer.getTotalSize()}px`, position: 'relative' }">
        <div
          v-for="row in virtualizer.getVirtualItems()"
          :key="filteredNodes[row.index].node_id"
          :class="['table-row', { selected: isSelected(filteredNodes[row.index]), alt: row.index % 2 === 1 }]"
          :style="{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            height: `${row.size}px`,
            transform: `translateY(${row.start}px)`,
          }"
          @click="selectNode(filteredNodes[row.index], $event)"
        >
          <div class="td mono col-id" :title="filteredNodes[row.index].node_id">{{ shortNodeId(filteredNodes[row.index].node_id) }}</div>
          <div class="td col-name" :title="filteredNodes[row.index].display_name">{{ filteredNodes[row.index].display_name }}</div>
          <div class="td mono col-value value-cell" :title="filteredNodes[row.index].value || ''">{{ filteredNodes[row.index].value || '—' }}</div>
          <div class="td col-quality"><span :style="{ color: qualityColor(filteredNodes[row.index].quality) }">{{ filteredNodes[row.index].quality || '—' }}</span></div>
          <div class="td mono col-time">{{ filteredNodes[row.index].timestamp ? filteredNodes[row.index].timestamp!.substring(11, 19) : '—' }}</div>
          <div class="td col-mode"><span class="mode-badge">{{ filteredNodes[row.index].access_mode === 'Subscription' ? 'Sub' : 'Poll' }}</span></div>
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

.col-id { flex: 2; min-width: 100px; }
.col-name { flex: 3; min-width: 120px; }
.col-value { flex: 2; min-width: 80px; }
.col-quality { flex: 0 0 55px; }
.col-time { flex: 0 0 70px; }
.col-mode { flex: 0 0 45px; }

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
