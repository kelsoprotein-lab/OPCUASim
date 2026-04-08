<script setup lang="ts">
import { ref, inject, watch, computed, onMounted, onUnmounted, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { LogEntry } from 'shared-frontend'

const props = defineProps<{
  expanded: boolean
}>()

const emit = defineEmits<{
  toggle: []
}>()

const selectedConnectionId = inject<Ref<string | null>>('selectedConnectionId')!

const logs = ref<LogEntry[]>([])
const directionFilter = ref<'all' | 'request' | 'response'>('all')
const searchQuery = ref('')
let refreshTimer: ReturnType<typeof setInterval> | null = null

async function loadLogs() {
  if (!selectedConnectionId.value) {
    logs.value = []
    return
  }
  try {
    logs.value = await invoke<LogEntry[]>('get_communication_logs', {
      conn_id: selectedConnectionId.value,
      since_seq: 0,
    })
  } catch (e) {
    console.error('Failed to load logs:', e)
  }
}

function startAutoRefresh() {
  stopAutoRefresh()
  if (props.expanded && selectedConnectionId.value) {
    refreshTimer = setInterval(loadLogs, 2000)
  }
}

function stopAutoRefresh() {
  if (refreshTimer) {
    clearInterval(refreshTimer)
    refreshTimer = null
  }
}

watch(() => props.expanded, (expanded) => {
  if (expanded) {
    loadLogs()
    startAutoRefresh()
  } else {
    stopAutoRefresh()
  }
})

watch(selectedConnectionId, () => {
  if (props.expanded) {
    loadLogs()
    startAutoRefresh()
  }
})

onMounted(() => {
  if (props.expanded) {
    loadLogs()
    startAutoRefresh()
  }
})

onUnmounted(stopAutoRefresh)

const filteredLogs = computed(() => {
  return logs.value.filter((log) => {
    if (directionFilter.value !== 'all' && log.direction.toLowerCase() !== directionFilter.value) {
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

async function clearLogs() {
  if (!selectedConnectionId.value) return
  try {
    await invoke('clear_communication_logs', { conn_id: selectedConnectionId.value })
    logs.value = []
  } catch (e) {
    console.error('Failed to clear logs:', e)
  }
}

async function exportCsv() {
  if (!selectedConnectionId.value) return
  try {
    const csv = await invoke<string>('export_logs_csv', { conn_id: selectedConnectionId.value })
    const blob = new Blob([csv], { type: 'text/csv' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `opcua-logs.csv`
    a.click()
    URL.revokeObjectURL(url)
  } catch (e) {
    console.error('Failed to export:', e)
  }
}

function directionColor(direction: string): string {
  return direction === 'Request' ? '#a6e3a1' : '#89b4fa'
}
</script>

<template>
  <div class="log-panel">
    <div class="log-header" @click="emit('toggle')">
      <span class="log-title">Communication Log</span>
      <span class="log-count">{{ filteredLogs.length }} / {{ logs.length }}</span>

      <template v-if="expanded">
        <div class="filter-group" @click.stop>
          <button
            :class="['filter-btn', { active: directionFilter === 'all' }]"
            @click="directionFilter = 'all'"
          >All</button>
          <button
            :class="['filter-btn', { active: directionFilter === 'request' }]"
            @click="directionFilter = 'request'"
          >Req</button>
          <button
            :class="['filter-btn', { active: directionFilter === 'response' }]"
            @click="directionFilter = 'response'"
          >Res</button>
        </div>

        <input
          v-model="searchQuery"
          class="search-input"
          placeholder="Search..."
          @click.stop
        />

        <button class="action-btn" @click.stop="clearLogs">Clear</button>
        <button class="action-btn" @click.stop="exportCsv">Export</button>
      </template>

      <span class="expand-icon">{{ expanded ? '▼' : '▲' }}</span>
    </div>

    <div v-if="expanded" class="log-body">
      <table class="log-table">
        <thead>
          <tr>
            <th style="width: 160px">Timestamp</th>
            <th style="width: 70px">Direction</th>
            <th style="width: 100px">Service</th>
            <th>Detail</th>
            <th style="width: 80px">Status</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(log, i) in filteredLogs" :key="i" :class="{ alt: i % 2 === 1 }">
            <td class="mono">{{ log.timestamp }}</td>
            <td :style="{ color: directionColor(log.direction) }">{{ log.direction }}</td>
            <td>{{ log.service }}</td>
            <td class="mono detail-cell">{{ log.detail }}</td>
            <td>{{ log.status || '' }}</td>
          </tr>
          <tr v-if="filteredLogs.length === 0">
            <td colspan="5" class="empty-row">No log entries</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.log-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.log-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  height: 32px;
  cursor: pointer;
  user-select: none;
  flex-shrink: 0;
}

.log-title {
  font-size: 12px;
  font-weight: 600;
  color: #cdd6f4;
}

.log-count {
  font-size: 11px;
  color: #585b70;
}

.filter-group {
  display: flex;
  gap: 2px;
  margin-left: 8px;
}

.filter-btn {
  background: none;
  border: 1px solid #313244;
  color: #585b70;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 3px;
  cursor: pointer;
}

.filter-btn.active {
  background: #313244;
  color: #cdd6f4;
}

.search-input {
  background: #11111b;
  border: 1px solid #313244;
  border-radius: 3px;
  color: #cdd6f4;
  font-size: 11px;
  padding: 2px 8px;
  width: 140px;
  outline: none;
}

.search-input:focus {
  border-color: #89b4fa;
}

.action-btn {
  background: none;
  border: 1px solid #313244;
  color: #a6adc8;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 3px;
  cursor: pointer;
}

.action-btn:hover {
  background: #313244;
}

.expand-icon {
  margin-left: auto;
  font-size: 10px;
  color: #585b70;
}

.log-body {
  flex: 1;
  overflow-y: auto;
}

.log-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}

.log-table th {
  text-align: left;
  padding: 4px 8px;
  color: #a6adc8;
  font-weight: 500;
  font-size: 11px;
  background: #181825;
  position: sticky;
  top: 0;
  border-bottom: 1px solid #313244;
}

.log-table td {
  padding: 3px 8px;
  color: #cdd6f4;
  border-bottom: 1px solid #181825;
}

.log-table tr.alt {
  background: #181825;
}

.mono {
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 11px;
}

.detail-cell {
  max-width: 400px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.empty-row {
  text-align: center;
  color: #585b70;
  padding: 12px !important;
}
</style>
