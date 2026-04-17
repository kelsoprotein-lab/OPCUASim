<script setup lang="ts">
import { ref } from 'vue'
import type { ServerStatus } from '../types'

defineProps<{
  status: ServerStatus
}>()

const emit = defineEmits<{
  start: []
  stop: []
  addFolder: [displayName: string, parentId: string]
  addNode: [params: { displayName: string; parentId: string; dataType: string; writable: boolean; simulation: Record<string, unknown> }]
}>()

const showAddNode = ref(false)
const newNodeName = ref('')
const newNodeDataType = ref('Double')
const newNodeWritable = ref(true)
const newNodeSimType = ref('Random')
const newNodeMin = ref(0)
const newNodeMax = ref(100)
const newNodeInterval = ref(1000)

function addFolder() {
  const name = prompt('Folder name:')
  if (name) {
    emit('addFolder', name, 'i=85')
  }
}

function addNode() {
  if (!newNodeName.value) return

  let simulation: Record<string, unknown>
  if (newNodeSimType.value === 'Static') {
    simulation = { type: 'Static', value: '0' }
  } else {
    simulation = {
      type: 'Random',
      min: newNodeMin.value,
      max: newNodeMax.value,
      interval_ms: newNodeInterval.value,
    }
  }

  emit('addNode', {
    displayName: newNodeName.value,
    parentId: 'i=85',
    dataType: newNodeDataType.value,
    writable: newNodeWritable.value,
    simulation,
  })

  newNodeName.value = ''
  showAddNode.value = false
}
</script>

<template>
  <div class="toolbar">
    <div class="toolbar-left">
      <button class="tool-btn" @click="addFolder" title="New Folder">+ Folder</button>
      <button class="tool-btn" @click="showAddNode = !showAddNode" title="New Node">+ Node</button>
    </div>
    <div class="toolbar-center">
      <span class="app-title">OPCUAServer</span>
    </div>
    <div class="toolbar-right">
      <button
        v-if="status.state === 'Stopped'"
        class="tool-btn start-btn"
        @click="$emit('start')"
      >Start</button>
      <button
        v-else
        class="tool-btn stop-btn"
        @click="$emit('stop')"
        :disabled="status.state === 'Starting' || status.state === 'Stopping'"
      >Stop</button>
    </div>

    <!-- Quick add node panel -->
    <div v-if="showAddNode" class="add-node-panel">
      <div class="add-node-row">
        <input v-model="newNodeName" placeholder="Display Name" class="add-input" />
        <select v-model="newNodeDataType" class="add-select">
          <option>Boolean</option><option>Int16</option><option>Int32</option>
          <option>Int64</option><option>UInt16</option><option>UInt32</option>
          <option>UInt64</option><option>Float</option><option>Double</option>
          <option>String</option>
        </select>
      </div>
      <div class="add-node-row">
        <select v-model="newNodeSimType" class="add-select">
          <option>Static</option><option>Random</option>
        </select>
        <template v-if="newNodeSimType === 'Random'">
          <input v-model.number="newNodeMin" type="number" placeholder="Min" class="add-input-sm" />
          <input v-model.number="newNodeMax" type="number" placeholder="Max" class="add-input-sm" />
          <input v-model.number="newNodeInterval" type="number" placeholder="ms" class="add-input-sm" />
        </template>
        <label class="add-check"><input type="checkbox" v-model="newNodeWritable" /> Writable</label>
        <button class="tool-btn" @click="addNode">Add</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  padding: 0 12px;
  background: #181825;
  border-bottom: 1px solid #313244;
  position: relative;
}

.toolbar-left, .toolbar-right {
  display: flex;
  gap: 6px;
}

.toolbar-center {
  flex: 1;
  text-align: center;
}

.app-title {
  font-size: 13px;
  font-weight: 600;
  color: #585b70;
}

.tool-btn {
  padding: 4px 12px;
  background: #313244;
  color: #cdd6f4;
  border: none;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
}

.tool-btn:hover:not(:disabled) { background: #45475a; }
.tool-btn:disabled { opacity: 0.4; cursor: not-allowed; }

.start-btn { background: #1e4d2b; color: #a6e3a1; }
.start-btn:hover { background: #2a6b3a; }
.stop-btn { background: #4d1e1e; color: #f38ba8; }
.stop-btn:hover { background: #6b2a2a; }

.add-node-panel {
  position: absolute;
  top: 42px;
  left: 0;
  right: 0;
  background: #1e1e2e;
  border-bottom: 1px solid #313244;
  padding: 8px 12px;
  z-index: 10;
}

.add-node-row {
  display: flex;
  gap: 6px;
  margin-bottom: 6px;
  align-items: center;
}

.add-input {
  flex: 1;
  padding: 4px 8px;
  background: #11111b;
  border: 1px solid #313244;
  border-radius: 4px;
  color: #cdd6f4;
  font-size: 12px;
  outline: none;
}

.add-input:focus { border-color: #89b4fa; }

.add-input-sm {
  width: 60px;
  padding: 4px 6px;
  background: #11111b;
  border: 1px solid #313244;
  border-radius: 4px;
  color: #cdd6f4;
  font-size: 12px;
  outline: none;
}

.add-select {
  padding: 4px 8px;
  background: #11111b;
  border: 1px solid #313244;
  border-radius: 4px;
  color: #cdd6f4;
  font-size: 12px;
}

.add-check {
  font-size: 12px;
  color: #585b70;
  display: flex;
  align-items: center;
  gap: 4px;
}
</style>
