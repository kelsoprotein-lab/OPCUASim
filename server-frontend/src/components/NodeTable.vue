<script setup lang="ts">
import type { ServerNodeInfo } from '../types'

defineProps<{
  nodes: ServerNodeInfo[]
  selectedNodeId: string | null
}>()

const emit = defineEmits<{
  select: [nodeId: string | null]
}>()

function simModeLabel(sim: ServerNodeInfo['simulation']): string {
  if (!sim) return '—'
  return sim.type
}
</script>

<template>
  <div class="node-table">
    <div class="panel-header">NODES</div>
    <div class="table-header">
      <span class="col col-name">Name</span>
      <span class="col col-type">DataType</span>
      <span class="col col-sim">SimMode</span>
      <span class="col col-value">Value</span>
      <span class="col col-rw">RW</span>
    </div>
    <div class="table-body">
      <div
        v-for="node in nodes"
        :key="node.node_id"
        class="table-row"
        :class="{ selected: selectedNodeId === node.node_id }"
        @click="emit('select', node.node_id)"
      >
        <span class="col col-name" :title="node.node_id">{{ node.display_name }}</span>
        <span class="col col-type">{{ node.data_type }}</span>
        <span class="col col-sim">{{ simModeLabel(node.simulation) }}</span>
        <span class="col col-value mono">{{ node.current_value || '—' }}</span>
        <span class="col col-rw">{{ node.writable ? 'RW' : 'R' }}</span>
      </div>
      <div v-if="nodes.length === 0" class="empty-hint">
        No variable nodes configured
      </div>
    </div>
  </div>
</template>

<style scoped>
.node-table {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-header {
  font-size: 11px;
  font-weight: 600;
  color: #585b70;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 8px 12px;
  border-bottom: 1px solid #313244;
}

.table-header {
  display: flex;
  padding: 4px 12px;
  background: #181825;
  border-bottom: 1px solid #313244;
  font-size: 11px;
  color: #585b70;
  font-weight: 600;
}

.table-body {
  flex: 1;
  overflow-y: auto;
}

.table-row {
  display: flex;
  padding: 4px 12px;
  font-size: 12px;
  cursor: pointer;
  border-bottom: 1px solid #11111b;
}

.table-row:hover { background: #313244; }
.table-row.selected { background: #45475a; }

.col { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.col-name { flex: 2; }
.col-type { flex: 1; }
.col-sim { flex: 1; }
.col-value { flex: 1; font-family: 'SF Mono', monospace; font-size: 11px; }
.col-rw { width: 32px; flex-shrink: 0; text-align: center; }

.mono { font-family: 'SF Mono', 'Fira Code', monospace; }

.empty-hint {
  padding: 20px;
  color: #585b70;
  font-size: 12px;
  text-align: center;
}
</style>
