import { ref, computed, type Ref } from 'vue'
import type { LogEntry } from '../types/common'

export type DirectionFilter = 'all' | 'request' | 'response'

export function useLogFilter(logs: Ref<LogEntry[]>) {
  const directionFilter = ref<DirectionFilter>('all')
  const serviceFilter = ref<string>('all')
  const searchQuery = ref('')

  const availableServices = computed(() => {
    const services = new Set<string>()
    logs.value.forEach((log) => {
      if (log.service) services.add(log.service)
    })
    return Array.from(services).sort()
  })

  const filteredLogs = computed(() => {
    return logs.value.filter((log) => {
      if (directionFilter.value !== 'all' && log.direction.toLowerCase() !== directionFilter.value) {
        return false
      }
      if (serviceFilter.value !== 'all' && log.service !== serviceFilter.value) {
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

  const filterSummary = computed(() => {
    const total = logs.value.length
    const shown = filteredLogs.value.length
    return total === shown ? `${total} entries` : `${shown} / ${total} entries`
  })

  function resetFilters() {
    directionFilter.value = 'all'
    serviceFilter.value = 'all'
    searchQuery.value = ''
  }

  return {
    directionFilter,
    serviceFilter,
    searchQuery,
    availableServices,
    filteredLogs,
    filterSummary,
    resetFilters,
  }
}
