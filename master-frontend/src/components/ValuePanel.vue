<script setup lang="ts">
import { inject, computed, ref, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { MonitoredNodeInfo } from '../types'
import { dialogKey } from '../composables/useDialog'

const selectedNodes = inject<Ref<MonitoredNodeInfo[]>>('selectedNodes')!
const selectedConnectionId = inject<Ref<string | null>>('selectedConnectionId')!
const dialog = inject(dialogKey)!

const node = computed(() => selectedNodes.value.length === 1 ? selectedNodes.value[0] : null)
const multiCount = computed(() => selectedNodes.value.length)
const isWritable = computed(() => node.value ? (node.value.user_access_level & 0x02) !== 0 : false)

const writeValue = ref('')

async function readNode() {
  if (!node.value || !selectedConnectionId.value) return
  try {
    await invoke('read_node_attributes', {
      connId: selectedConnectionId.value,
      nodeId: node.value.node_id,
    }).catch(() => {
      // Not yet implemented until Task 8
    })
  } catch (e) {
    await dialog.showAlert('Read Error', String(e))
  }
}

async function writeNode() {
  if (!node.value || !selectedConnectionId.value || !writeValue.value) return
  try {
    await invoke('write_node_value', {
      connId: selectedConnectionId.value,
      nodeId: node.value.node_id,
      value: writeValue.value,
      dataType: node.value.data_type,
    })
    writeValue.value = ''
  } catch (e) {
    await dialog.showAlert('Write Error', String(e))
  }
}

function qualityColor(quality?: string): string {
  if (!quality) return '#585b70'
  if (quality === 'Good') return '#a6e3a1'
  if (quality.startsWith('Bad')) return '#f38ba8'
  return '#f9e2af'
}
</script>

<template>
  <div class="value-panel">
    <template v-if="node">
      <div class="section">
        <div class="section-header">NODE INFO</div>
        <div class="field">
          <span class="field-label">NodeId</span>
          <span class="field-value mono">{{ node.node_id }}</span>
        </div>
        <div class="field">
          <span class="field-label">Name</span>
          <span class="field-value">{{ node.display_name }}</span>
        </div>
        <div class="field">
          <span class="field-label">Data Type</span>
          <span class="field-value">{{ node.data_type || '—' }}</span>
        </div>
        <div class="field">
          <span class="field-label">Access Mode</span>
          <span class="field-value">{{ node.access_mode }} ({{ node.interval_ms }}ms)</span>
        </div>
        <div class="field">
          <span class="field-label">Access Level</span>
          <span class="field-value">{{ node.user_access_level }} ({{ (node.user_access_level & 0x01) ? 'R' : '' }}{{ (node.user_access_level & 0x02) ? 'W' : '' }}{{ node.user_access_level === 0 ? 'unknown' : '' }})</span>
        </div>
      </div>

      <div class="section">
        <div class="section-header">CURRENT VALUE</div>
        <div class="value-display">
          <span class="current-value mono">{{ node.value || '—' }}</span>
        </div>
        <div class="field">
          <span class="field-label">Quality</span>
          <span class="field-value" :style="{ color: qualityColor(node.quality) }">{{ node.quality || '—' }}</span>
        </div>
        <div class="field">
          <span class="field-label">Timestamp</span>
          <span class="field-value mono">{{ node.timestamp || '—' }}</span>
        </div>
      </div>

      <div class="section">
        <div class="section-header">ACTIONS</div>
        <button class="action-btn" @click="readNode">Read Value</button>
        <div class="write-group">
          <input
            v-model="writeValue"
            class="write-input"
            placeholder="Value to write..."
          />
          <button class="action-btn" @click="writeNode" :disabled="!writeValue">Write</button>
        </div>
        <div v-if="!isWritable" class="hint">Node reports read-only (level={{ node.user_access_level }})</div>
      </div>
    </template>

    <template v-else-if="multiCount > 1">
      <div class="multi-select">
        {{ multiCount }} nodes selected
      </div>
    </template>

    <template v-else>
      <div class="no-selection">
        Select a node to view details
      </div>
    </template>
  </div>
</template>

<style scoped>
.value-panel {
  padding: 12px;
  height: 100%;
}

.section {
  margin-bottom: 16px;
}

.section-header {
  font-size: 11px;
  font-weight: 600;
  color: #585b70;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 8px;
  padding-bottom: 4px;
  border-bottom: 1px solid #313244;
}

.field {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  padding: 3px 0;
  gap: 8px;
}

.field-label {
  font-size: 12px;
  color: #585b70;
  flex-shrink: 0;
}

.field-value {
  font-size: 12px;
  color: #cdd6f4;
  text-align: right;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mono {
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 11px;
}

.value-display {
  background: #11111b;
  border: 1px solid #313244;
  border-radius: 4px;
  padding: 8px 10px;
  margin-bottom: 8px;
}

.current-value {
  font-size: 16px;
  color: #cdd6f4;
  word-break: break-all;
}

.action-btn {
  width: 100%;
  padding: 6px 12px;
  background: #313244;
  color: #cdd6f4;
  border: none;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  margin-bottom: 6px;
}

.action-btn:hover:not(:disabled) {
  background: #45475a;
}

.action-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.write-group {
  display: flex;
  gap: 6px;
}

.write-input {
  flex: 1;
  padding: 6px 10px;
  background: #11111b;
  border: 1px solid #313244;
  border-radius: 4px;
  color: #cdd6f4;
  font-size: 12px;
  outline: none;
}

.write-input:focus {
  border-color: #89b4fa;
}

.write-group .action-btn {
  width: auto;
  margin-bottom: 0;
}

.hint {
  font-size: 11px;
  color: #f9e2af;
  margin-top: 2px;
}

.no-selection, .multi-select {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #585b70;
  font-size: 13px;
}
</style>
