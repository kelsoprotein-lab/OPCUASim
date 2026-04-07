import { ref } from 'vue'

export interface Toast {
  id: number
  message: string
  level: 'error' | 'warning' | 'info'
  persistent: boolean
  timestamp: number
}

let nextId = 0

export function useErrorHandler() {
  const toasts = ref<Toast[]>([])

  function categorize(error: unknown): { message: string; level: 'error' | 'warning' | 'info'; persistent: boolean } {
    const msg = typeof error === 'string' ? error : String(error)

    if (msg.includes('ConnectionFailed') || msg.includes('SessionTimeout') || msg.includes('SecurityRejected')) {
      return { message: msg, level: 'error', persistent: true }
    }
    return { message: msg, level: 'error', persistent: false }
  }

  function handleError(error: unknown) {
    const { message, level, persistent } = categorize(error)
    const toast: Toast = { id: nextId++, message, level, persistent, timestamp: Date.now() }
    toasts.value.push(toast)

    if (!persistent) {
      setTimeout(() => removeToast(toast.id), 3000)
    }
  }

  function removeToast(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id)
  }

  function clearToasts() {
    toasts.value = []
  }

  return { toasts, handleError, removeToast, clearToasts }
}
