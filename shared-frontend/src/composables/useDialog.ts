import { reactive, type InjectionKey } from 'vue'
import type { DialogMode, DialogState } from '../types/common'

const state = reactive<DialogState>({
  visible: false,
  mode: 'alert',
  title: '',
  message: '',
  inputValue: '',
  resolve: null,
})

export const dialogKey: InjectionKey<{
  showAlert: (title: string, message?: string) => Promise<void>
  showConfirm: (title: string, message?: string) => Promise<boolean>
  showPrompt: (title: string, message?: string, defaultValue?: string) => Promise<string | null>
}> = Symbol('dialog')

function show(mode: DialogMode, title: string, message = '', defaultValue = ''): Promise<any> {
  return new Promise((resolve) => {
    state.mode = mode
    state.title = title
    state.message = message
    state.inputValue = defaultValue
    state.resolve = resolve
    state.visible = true
  })
}

export function showAlert(title: string, message = ''): Promise<void> {
  return show('alert', title, message).then(() => {})
}

export function showConfirm(title: string, message = ''): Promise<boolean> {
  return show('confirm', title, message)
}

export function showPrompt(title: string, message = '', defaultValue = ''): Promise<string | null> {
  return show('prompt', title, message, defaultValue)
}

export function dialogConfirm() {
  if (state.resolve) {
    if (state.mode === 'alert') state.resolve(true)
    else if (state.mode === 'confirm') state.resolve(true)
    else state.resolve(state.inputValue)
  }
  state.visible = false
}

export function dialogCancel() {
  if (state.resolve) {
    if (state.mode === 'confirm') state.resolve(false)
    else if (state.mode === 'prompt') state.resolve(null)
  }
  state.visible = false
}

export function useDialogState() {
  return state
}
