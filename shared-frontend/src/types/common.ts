export interface LogEntry {
  timestamp: string
  direction: string
  service: string
  detail: string
  status?: string
}

export type DialogMode = 'alert' | 'confirm' | 'prompt'

export interface DialogState {
  visible: boolean
  mode: DialogMode
  title: string
  message: string
  inputValue: string
  resolve: ((value: boolean | string | null) => void) | null
}
