<script setup lang="ts">
import type { ServerFolderInfo, ServerNodeInfo } from '../types'
import { computed } from 'vue'

const props = defineProps<{
  folders: ServerFolderInfo[]
  nodes: ServerNodeInfo[]
  selectedNodeId: string | null
}>()

const emit = defineEmits<{
  select: [nodeId: string | null]
  remove: [nodeId: string]
}>()

interface TreeItem {
  id: string
  name: string
  type: 'folder' | 'variable'
  children: TreeItem[]
}

const tree = computed(() => {
  const map = new Map<string, TreeItem>()

  // Create folder items
  for (const f of props.folders) {
    map.set(f.node_id, { id: f.node_id, name: f.display_name, type: 'folder', children: [] })
  }

  // Create variable items
  for (const n of props.nodes) {
    map.set(n.node_id, { id: n.node_id, name: n.display_name, type: 'variable', children: [] })
  }

  // Build hierarchy
  const roots: TreeItem[] = []
  const allItems = [...props.folders, ...props.nodes]
  for (const item of allItems) {
    const id = 'node_id' in item ? item.node_id : ''
    const parentId = 'parent_id' in item ? item.parent_id : ''
    const treeItem = map.get(id)
    if (!treeItem) continue

    const parent = map.get(parentId)
    if (parent) {
      parent.children.push(treeItem)
    } else {
      roots.push(treeItem)
    }
  }

  return roots
})

function onContextMenu(e: MouseEvent, nodeId: string) {
  e.preventDefault()
  if (confirm(`Delete "${nodeId}"?`)) {
    emit('remove', nodeId)
  }
}
</script>

<template>
  <div class="tree-panel">
    <div class="panel-header">ADDRESS SPACE</div>
    <div class="tree-content">
      <template v-if="tree.length === 0">
        <div class="empty-hint">No nodes. Use toolbar to add.</div>
      </template>
      <template v-for="item in tree" :key="item.id">
        <div
          class="tree-item"
          :class="{ selected: selectedNodeId === item.id, variable: item.type === 'variable' }"
          @click="emit('select', item.id)"
          @contextmenu="onContextMenu($event, item.id)"
        >
          <span class="icon">{{ item.type === 'folder' ? '📁' : '📊' }}</span>
          <span class="name">{{ item.name }}</span>
        </div>
        <div v-for="child in item.children" :key="child.id"
          class="tree-item child"
          :class="{ selected: selectedNodeId === child.id, variable: child.type === 'variable' }"
          @click="emit('select', child.id)"
          @contextmenu="onContextMenu($event, child.id)"
        >
          <span class="icon">{{ child.type === 'folder' ? '📁' : '📊' }}</span>
          <span class="name">{{ child.name }}</span>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.tree-panel {
  background: #181825;
  border-right: 1px solid #313244;
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

.tree-content {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.tree-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  cursor: pointer;
  font-size: 12px;
}

.tree-item:hover { background: #313244; }
.tree-item.selected { background: #45475a; }
.tree-item.child { padding-left: 28px; }

.icon { font-size: 14px; }
.name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.empty-hint {
  padding: 20px 12px;
  color: #585b70;
  font-size: 12px;
  text-align: center;
}
</style>
