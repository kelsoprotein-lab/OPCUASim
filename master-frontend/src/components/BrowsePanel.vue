<script setup lang="ts">
import { ref, inject, watch, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { BrowseResult } from '../types'
import { dialogKey } from '../composables/useDialog'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const selectedConnectionId = inject<Ref<string | null>>('selectedConnectionId')!
const refreshData = inject<() => void>('refreshData')!
const dialog = inject(dialogKey)!

interface TreeNode {
  nodeId: string
  displayName: string
  nodeClass: string
  dataType?: string
  hasChildren: boolean
  children: TreeNode[]
  expanded: boolean
  loaded: boolean
  checked: boolean
}

const rootNodes = ref<TreeNode[]>([])
const loading = ref(false)
const accessMode = ref<'subscription' | 'polling'>('subscription')
const intervalMs = ref(1000)

async function loadRootNodes() {
  if (!selectedConnectionId.value) return
  loading.value = true
  try {
    const results = await invoke<BrowseResult[]>('browse_root', {
      connId: selectedConnectionId.value,
    })
    rootNodes.value = results.map(toTreeNode)
  } catch (e) {
    console.error('Browse failed:', e)
    // For now, show a hint that browse is not yet implemented
    rootNodes.value = []
  } finally {
    loading.value = false
  }
}

function toTreeNode(r: BrowseResult): TreeNode {
  return {
    nodeId: r.node_id,
    displayName: r.display_name,
    nodeClass: r.node_class,
    dataType: r.data_type,
    hasChildren: r.has_children,
    children: [],
    expanded: false,
    loaded: false,
    checked: false,
  }
}

async function toggleExpand(node: TreeNode) {
  if (!node.hasChildren) return
  if (!node.loaded) {
    try {
      const results = await invoke<BrowseResult[]>('browse_node', {
        connId: selectedConnectionId.value,
        nodeId: node.nodeId,
      })
      node.children = results.map(toTreeNode)
      node.loaded = true
    } catch (e) {
      console.error('Browse node failed:', e)
    }
  }
  node.expanded = !node.expanded
}

function getCheckedNodes(): TreeNode[] {
  const checked: TreeNode[] = []
  function walk(nodes: TreeNode[]) {
    for (const n of nodes) {
      if (n.checked) checked.push(n)
      walk(n.children)
    }
  }
  walk(rootNodes.value)
  return checked
}

async function addToMonitoring() {
  const checked = getCheckedNodes()
  if (checked.length === 0) {
    await dialog.showAlert('No Selection', 'Please select at least one node.')
    return
  }

  try {
    await invoke('add_monitored_nodes', {
      connId: selectedConnectionId.value,
      nodes: checked.map((n) => ({
        node_id: n.nodeId,
        display_name: n.displayName,
        data_type: n.dataType || 'Unknown',
      })),
      accessMode: accessMode.value,
      intervalMs: intervalMs.value,
    })
    refreshData()
    emit('close')
  } catch (e) {
    await dialog.showAlert('Error', String(e))
  }
}

// Load root when opened
watch(() => props.visible, (v) => {
  if (v) loadRootNodes()
})
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="browse-overlay" @click.self="emit('close')">
      <div class="browse-dialog">
        <div class="browse-header">
          <span class="browse-title">Browse Server Nodes</span>
          <button class="close-btn" @click="emit('close')">✕</button>
        </div>

        <div class="browse-body">
          <div v-if="loading" class="loading-hint">Loading...</div>
          <div v-else-if="rootNodes.length === 0" class="loading-hint">
            No nodes found. The server may not be connected or browse is not yet implemented.
          </div>
          <div v-else class="node-tree">
            <template v-for="node in rootNodes" :key="node.nodeId">
              <div class="tree-row" :style="{ paddingLeft: '12px' }">
                <span
                  class="expand-arrow"
                  :class="{ invisible: !node.hasChildren }"
                  @click="toggleExpand(node)"
                >{{ node.expanded ? '▾' : '▸' }}</span>
                <input type="checkbox" v-model="node.checked" class="node-check" />
                <span class="node-name" @click="toggleExpand(node)">{{ node.displayName }}</span>
                <span class="node-class">{{ node.nodeClass }}</span>
                <span v-if="node.dataType" class="node-type">{{ node.dataType }}</span>
              </div>
              <template v-if="node.expanded">
                <div
                  v-for="child in node.children"
                  :key="child.nodeId"
                  class="tree-row"
                  :style="{ paddingLeft: '32px' }"
                >
                  <span
                    class="expand-arrow"
                    :class="{ invisible: !child.hasChildren }"
                    @click="toggleExpand(child)"
                  >{{ child.expanded ? '▾' : '▸' }}</span>
                  <input type="checkbox" v-model="child.checked" class="node-check" />
                  <span class="node-name" @click="toggleExpand(child)">{{ child.displayName }}</span>
                  <span class="node-class">{{ child.nodeClass }}</span>
                  <span v-if="child.dataType" class="node-type">{{ child.dataType }}</span>
                </div>
              </template>
            </template>
          </div>
        </div>

        <div class="browse-footer">
          <div class="mode-select">
            <label>
              <input type="radio" v-model="accessMode" value="subscription" /> Subscription
            </label>
            <label>
              <input type="radio" v-model="accessMode" value="polling" /> Polling
            </label>
            <label class="interval-label">
              Interval:
              <input type="number" v-model.number="intervalMs" class="interval-input" min="100" step="100" /> ms
            </label>
          </div>
          <div class="footer-actions">
            <button class="btn btn-cancel" @click="emit('close')">Cancel</button>
            <button class="btn btn-confirm" @click="addToMonitoring">Add to Monitoring</button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.browse-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9998;
}

.browse-dialog {
  background: #1e1e2e;
  border: 1px solid #313244;
  border-radius: 8px;
  width: 600px;
  height: 500px;
  display: flex;
  flex-direction: column;
}

.browse-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid #313244;
}

.browse-title {
  font-size: 14px;
  font-weight: 600;
  color: #cdd6f4;
}

.close-btn {
  background: none;
  border: none;
  color: #585b70;
  font-size: 16px;
  cursor: pointer;
}

.close-btn:hover {
  color: #cdd6f4;
}

.browse-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.loading-hint {
  text-align: center;
  color: #585b70;
  padding: 20px;
  font-size: 13px;
}

.tree-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  font-size: 13px;
}

.tree-row:hover {
  background: #313244;
}

.expand-arrow {
  width: 14px;
  text-align: center;
  color: #585b70;
  cursor: pointer;
  font-size: 12px;
  flex-shrink: 0;
}

.expand-arrow.invisible {
  visibility: hidden;
}

.node-check {
  accent-color: #89b4fa;
  flex-shrink: 0;
}

.node-name {
  color: #cdd6f4;
  cursor: pointer;
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.node-class {
  color: #585b70;
  font-size: 11px;
  flex-shrink: 0;
}

.node-type {
  color: #89b4fa;
  font-size: 11px;
  flex-shrink: 0;
}

.browse-footer {
  border-top: 1px solid #313244;
  padding: 12px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.mode-select {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 12px;
  color: #a6adc8;
}

.mode-select label {
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
}

.mode-select input[type="radio"] {
  accent-color: #89b4fa;
}

.interval-label {
  display: flex;
  align-items: center;
  gap: 4px;
}

.interval-input {
  width: 70px;
  background: #11111b;
  border: 1px solid #313244;
  border-radius: 3px;
  color: #cdd6f4;
  font-size: 12px;
  padding: 2px 6px;
  outline: none;
}

.interval-input:focus {
  border-color: #89b4fa;
}

.footer-actions {
  display: flex;
  gap: 8px;
}

.btn {
  padding: 6px 16px;
  border: none;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
}

.btn-cancel {
  background: #313244;
  color: #cdd6f4;
}

.btn-cancel:hover {
  background: #45475a;
}

.btn-confirm {
  background: #89b4fa;
  color: #1e1e2e;
}

.btn-confirm:hover {
  background: #74c7ec;
}
</style>
