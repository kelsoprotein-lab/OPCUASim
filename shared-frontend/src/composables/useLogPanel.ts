import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { LogEntry } from '../types/common'

export function useLogPanel() {
  const logs = ref<LogEntry[]>([])
  const loading = ref(false)

  async function loadLogs(connectionId: string | null) {
    if (!connectionId) {
      logs.value = []
      return
    }
    loading.value = true
    try {
      logs.value = await invoke<LogEntry[]>('get_communication_logs', {
        connId: connectionId,
        sinceSeq: 0,
      })
    } catch (e) {
      console.error('Failed to load logs:', e)
    } finally {
      loading.value = false
    }
  }

  async function clearLogs(connectionId: string | null) {
    if (!connectionId) return
    try {
      await invoke('clear_communication_logs', { connId: connectionId })
      logs.value = []
    } catch (e) {
      console.error('Failed to clear logs:', e)
    }
  }

  async function exportLogsCsv(connectionId: string | null) {
    if (!connectionId) return
    try {
      const csv = await invoke<string>('export_logs_csv', { connId: connectionId })
      const blob = new Blob([csv], { type: 'text/csv' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `opcua-logs-${connectionId}.csv`
      a.click()
      URL.revokeObjectURL(url)
    } catch (e) {
      console.error('Failed to export logs:', e)
    }
  }

  return { logs, loading, loadLogs, clearLogs, exportLogsCsv }
}
