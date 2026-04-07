<script setup lang="ts">
import { useDialogState, dialogConfirm, dialogCancel } from '../composables/useDialog'
import { nextTick, watch, ref } from 'vue'

const state = useDialogState()
const inputRef = ref<HTMLInputElement | null>(null)

watch(() => state.visible, async (visible) => {
  if (visible && state.mode === 'prompt') {
    await nextTick()
    inputRef.value?.focus()
    inputRef.value?.select()
  }
})

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') dialogConfirm()
  else if (e.key === 'Escape') dialogCancel()
}
</script>

<template>
  <Teleport to="body">
    <div v-if="state.visible" class="dialog-overlay" @keydown="onKeydown">
      <div class="dialog-box">
        <div class="dialog-title">{{ state.title }}</div>
        <div v-if="state.message" class="dialog-message">{{ state.message }}</div>
        <input
          v-if="state.mode === 'prompt'"
          ref="inputRef"
          v-model="state.inputValue"
          class="dialog-input"
          @keydown.enter="dialogConfirm"
        />
        <div class="dialog-buttons">
          <button v-if="state.mode !== 'alert'" class="btn btn-cancel" @click="dialogCancel">Cancel</button>
          <button class="btn btn-confirm" @click="dialogConfirm">OK</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.dialog-box {
  background: #1e1e2e;
  border: 1px solid #313244;
  border-radius: 8px;
  padding: 20px;
  min-width: 320px;
  max-width: 480px;
}

.dialog-title {
  font-size: 14px;
  font-weight: 600;
  color: #cdd6f4;
  margin-bottom: 8px;
}

.dialog-message {
  font-size: 13px;
  color: #a6adc8;
  margin-bottom: 12px;
}

.dialog-input {
  width: 100%;
  padding: 6px 10px;
  background: #11111b;
  border: 1px solid #313244;
  border-radius: 4px;
  color: #cdd6f4;
  font-size: 13px;
  outline: none;
  margin-bottom: 12px;
}

.dialog-input:focus {
  border-color: #89b4fa;
}

.dialog-buttons {
  display: flex;
  justify-content: flex-end;
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
