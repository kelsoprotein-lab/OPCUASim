<script setup lang="ts">
import { ref, inject, watch, onMounted, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ConnectionInfo, NodeGroupInfo } from '../types'

const emit = defineEmits<{
  'connection-select': [id: string, state: string]
}>()

const selectedConnectionId = inject<Ref<string | null>>('selectedConnectionId')!
const treeRefreshKey = inject<Ref<number>>('treeRefreshKey')!

const connections = ref<ConnectionInfo[]>([])
const groups = ref<NodeGroupInfo[]>([])
const activeTab = ref<'connections' | 'groups'>('connections')

async function loadData() {
  try {
    connections.value = await invoke<ConnectionInfo[]>('list_connections')
    groups.value = await invoke<NodeGroupInfo[]>('list_groups')
  } catch (e) {
    console.error('Failed to load tree data:', e)
  }
}

onMounted(loadData)
watch(treeRefreshKey, loadData)

function selectConnection(conn: ConnectionInfo) {
  emit('connection-select', conn.id, conn.state)
}

function stateColor(state: string): string {
  switch (state) {
    case 'Connected': return '#a6e3a1'
    case 'Connecting':
    case 'Reconnecting': return '#f9e2af'
    default: return '#585b70'
  }
}
</script>

<template>
  <div class="tree-container">
    <div class="tabs">
      <button
        :class="['tab', { active: activeTab === 'connections' }]"
        @click="activeTab = 'connections'"
      >Connections</button>
      <button
        :class="['tab', { active: activeTab === 'groups' }]"
        @click="activeTab = 'groups'"
      >Groups</button>
    </div>

    <div v-if="activeTab === 'connections'" class="tree-list">
      <div v-if="connections.length === 0" class="empty-hint">No connections</div>
      <div
        v-for="conn in connections"
        :key="conn.id"
        :class="['tree-item', { selected: selectedConnectionId === conn.id }]"
        @click="selectConnection(conn)"
      >
        <span class="state-dot" :style="{ background: stateColor(conn.state) }" />
        <div class="item-content">
          <div class="item-name">{{ conn.name }}</div>
          <div class="item-detail">{{ conn.endpoint_url }}</div>
        </div>
      </div>
    </div>

    <div v-if="activeTab === 'groups'" class="tree-list">
      <div v-if="groups.length === 0" class="empty-hint">No groups</div>
      <div
        v-for="group in groups"
        :key="group.id"
        class="tree-item"
      >
        <div class="item-content">
          <div class="item-name">{{ group.name }}</div>
          <div class="item-detail">{{ group.node_count }} nodes</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tree-container {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.tabs {
  display: flex;
  border-bottom: 1px solid #313244;
}

.tab {
  flex: 1;
  padding: 8px 0;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: #585b70;
  font-size: 12px;
  cursor: pointer;
  text-align: center;
}

.tab.active {
  color: #cdd6f4;
  border-bottom-color: #89b4fa;
}

.tab:hover {
  color: #a6adc8;
}

.tree-list {
  flex: 1;
  overflow-y: auto;
}

.empty-hint {
  padding: 12px;
  color: #585b70;
  font-size: 12px;
  text-align: center;
}

.tree-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  border-left: 2px solid transparent;
}

.tree-item:hover {
  background: #313244;
}

.tree-item.selected {
  background: #313244;
  border-left-color: #89b4fa;
}

.state-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.item-content {
  min-width: 0;
}

.item-name {
  font-size: 13px;
  color: #cdd6f4;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-detail {
  font-size: 11px;
  color: #585b70;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
