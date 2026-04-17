<script setup lang="ts">
import type { ServerNodeInfo } from '../types'

defineProps<{
  node: ServerNodeInfo | null
}>()

function simDescription(sim: ServerNodeInfo['simulation']): string {
  if (!sim) return '—'
  switch (sim.type) {
    case 'Static': return `Static: ${sim.value}`
    case 'Random': return `Random [${sim.min}, ${sim.max}] @ ${sim.interval_ms}ms`
    case 'Sine': return `Sine amp=${sim.amplitude} off=${sim.offset} T=${sim.period_ms}ms @ ${sim.interval_ms}ms`
    case 'Linear': return `Linear ${sim.start}→ step=${sim.step} [${sim.min}, ${sim.max}] ${sim.mode} @ ${sim.interval_ms}ms`
    case 'Script': return `Script: ${sim.expression} @ ${sim.interval_ms}ms`
    default: return '—'
  }
}
</script>

<template>
  <div class="property-editor">
    <div class="panel-header">PROPERTIES</div>
    <div v-if="node" class="props-content">
      <div class="section">
        <div class="section-title">Node Info</div>
        <div class="field">
          <span class="label">NodeId</span>
          <span class="value mono">{{ node.node_id }}</span>
        </div>
        <div class="field">
          <span class="label">Name</span>
          <span class="value">{{ node.display_name }}</span>
        </div>
        <div class="field">
          <span class="label">Parent</span>
          <span class="value mono">{{ node.parent_id }}</span>
        </div>
        <div class="field">
          <span class="label">DataType</span>
          <span class="value">{{ node.data_type }}</span>
        </div>
        <div class="field">
          <span class="label">Writable</span>
          <span class="value">{{ node.writable ? 'Yes' : 'No' }}</span>
        </div>
      </div>

      <div class="section">
        <div class="section-title">Simulation</div>
        <div class="field">
          <span class="label">Mode</span>
          <span class="value">{{ node.simulation?.type || '—' }}</span>
        </div>
        <div class="field">
          <span class="label">Config</span>
          <span class="value sim-desc">{{ simDescription(node.simulation) }}</span>
        </div>
      </div>

      <div class="section">
        <div class="section-title">Current Value</div>
        <div class="value-display mono">{{ node.current_value || '—' }}</div>
      </div>
    </div>
    <div v-else class="no-selection">
      Select a node to view properties
    </div>
  </div>
</template>

<style scoped>
.property-editor {
  background: #181825;
  border-left: 1px solid #313244;
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

.props-content {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
}

.section {
  margin-bottom: 16px;
}

.section-title {
  font-size: 11px;
  font-weight: 600;
  color: #585b70;
  margin-bottom: 6px;
  padding-bottom: 3px;
  border-bottom: 1px solid #313244;
}

.field {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  padding: 2px 0;
  gap: 8px;
}

.label {
  font-size: 12px;
  color: #585b70;
  flex-shrink: 0;
}

.value {
  font-size: 12px;
  color: #cdd6f4;
  text-align: right;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sim-desc {
  font-size: 11px;
  white-space: normal;
  word-break: break-word;
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
  font-size: 16px;
  color: #cdd6f4;
  word-break: break-all;
}

.no-selection {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #585b70;
  font-size: 13px;
}
</style>
