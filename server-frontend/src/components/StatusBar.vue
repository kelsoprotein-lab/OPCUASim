<script setup lang="ts">
import type { ServerStatus } from '../types'

defineProps<{
  status: ServerStatus
  nodeCount: number
  folderCount: number
}>()

function stateColor(state: string): string {
  switch (state) {
    case 'Running': return '#a6e3a1'
    case 'Starting':
    case 'Stopping': return '#f9e2af'
    case 'Stopped': return '#585b70'
    default: return '#585b70'
  }
}
</script>

<template>
  <div class="status-bar">
    <div class="status-left">
      <span class="status-dot" :style="{ background: stateColor(status.state) }"></span>
      <span class="status-text">{{ status.state }}</span>
    </div>
    <div class="status-right">
      <span class="status-item">Folders: {{ folderCount }}</span>
      <span class="status-item">Nodes: {{ nodeCount }}</span>
    </div>
  </div>
</template>

<style scoped>
.status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  background: #181825;
  font-size: 11px;
  color: #585b70;
}

.status-left {
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-text {
  color: #cdd6f4;
}

.status-right {
  display: flex;
  gap: 16px;
}

.status-item {
  color: #585b70;
}
</style>
