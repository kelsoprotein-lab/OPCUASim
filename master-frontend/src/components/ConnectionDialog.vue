<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
  submit: [config: ConnectionFormData]
}>()

export interface ConnectionFormData {
  name: string
  endpoint_url: string
  security_policy: string
  security_mode: string
  auth_type: string
  username: string
  password: string
  cert_path: string
  key_path: string
}

const form = ref<ConnectionFormData>({
  name: '',
  endpoint_url: 'opc.tcp://localhost:4840',
  security_policy: 'None',
  security_mode: 'None',
  auth_type: 'Anonymous',
  username: '',
  password: '',
  cert_path: '',
  key_path: '',
})

watch(() => props.visible, (v) => {
  if (v) {
    // Reset form
    form.value = {
      name: '',
      endpoint_url: 'opc.tcp://localhost:4840',
      security_policy: 'None',
      security_mode: 'None',
      auth_type: 'Anonymous',
      username: '',
      password: '',
      cert_path: '',
      key_path: '',
    }
  }
})

function onSubmit() {
  if (!form.value.name.trim()) {
    form.value.name = 'New Connection'
  }
  emit('submit', { ...form.value })
}
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="dialog-overlay" @click.self="emit('close')">
      <div class="dialog-box">
        <div class="dialog-title">Add Server</div>

        <!-- Server Information -->
        <div class="section">
          <div class="section-label">Server Information</div>
          <div class="field-row">
            <label>Configuration Name</label>
            <input v-model="form.name" class="field-input" placeholder="My OPC UA Server" />
          </div>
          <div class="field-row">
            <label>Endpoint URL</label>
            <input v-model="form.endpoint_url" class="field-input" placeholder="opc.tcp://localhost:4840" />
          </div>
        </div>

        <!-- Security Settings -->
        <div class="section">
          <div class="section-label">Security Settings</div>
          <div class="field-row">
            <label>Security Policy</label>
            <select v-model="form.security_policy" class="field-select">
              <option value="None">None</option>
              <option value="Basic128Rsa15">Basic128Rsa15</option>
              <option value="Basic256">Basic256</option>
              <option value="Basic256Sha256">Basic256Sha256</option>
              <option value="Aes128_Sha256_RsaOaep">Aes128_Sha256_RsaOaep</option>
              <option value="Aes256_Sha256_RsaPss">Aes256_Sha256_RsaPss</option>
            </select>
          </div>
          <div class="field-row">
            <label>Message Security Mode</label>
            <select v-model="form.security_mode" class="field-select">
              <option value="None">None</option>
              <option value="Sign">Sign</option>
              <option value="SignAndEncrypt">SignAndEncrypt</option>
            </select>
          </div>
        </div>

        <!-- Authentication Settings -->
        <div class="section">
          <div class="section-label">Authentication Settings</div>
          <div class="auth-options">
            <label class="radio-label">
              <input type="radio" v-model="form.auth_type" value="Anonymous" /> Anonymous
            </label>
            <label class="radio-label">
              <input type="radio" v-model="form.auth_type" value="UserPassword" /> Username / Password
            </label>
            <label class="radio-label">
              <input type="radio" v-model="form.auth_type" value="Certificate" /> Certificate
            </label>
          </div>

          <template v-if="form.auth_type === 'UserPassword'">
            <div class="field-row">
              <label>Username</label>
              <input v-model="form.username" class="field-input" />
            </div>
            <div class="field-row">
              <label>Password</label>
              <input v-model="form.password" type="password" class="field-input" />
            </div>
          </template>

          <template v-if="form.auth_type === 'Certificate'">
            <div class="field-row">
              <label>Certificate Path</label>
              <input v-model="form.cert_path" class="field-input" placeholder="/path/to/cert.pem" />
            </div>
            <div class="field-row">
              <label>Private Key Path</label>
              <input v-model="form.key_path" class="field-input" placeholder="/path/to/key.pem" />
            </div>
          </template>
        </div>

        <!-- Buttons -->
        <div class="dialog-buttons">
          <button class="btn btn-cancel" @click="emit('close')">Cancel</button>
          <button class="btn btn-confirm" @click="onSubmit">OK</button>
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
  width: 420px;
}

.dialog-title {
  font-size: 15px;
  font-weight: 600;
  color: #cdd6f4;
  margin-bottom: 16px;
}

.section {
  margin-bottom: 14px;
}

.section-label {
  font-size: 11px;
  font-weight: 600;
  color: #585b70;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 8px;
  padding-bottom: 4px;
  border-bottom: 1px solid #313244;
}

.field-row {
  display: flex;
  align-items: center;
  margin-bottom: 6px;
  gap: 8px;
}

.field-row label {
  font-size: 12px;
  color: #a6adc8;
  width: 130px;
  flex-shrink: 0;
}

.field-input, .field-select {
  flex: 1;
  padding: 5px 10px;
  background: #11111b;
  border: 1px solid #313244;
  border-radius: 4px;
  color: #cdd6f4;
  font-size: 12px;
  outline: none;
}

.field-input:focus, .field-select:focus {
  border-color: #89b4fa;
}

.field-select {
  appearance: auto;
}

.auth-options {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 8px;
}

.radio-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #cdd6f4;
  cursor: pointer;
}

.radio-label input[type="radio"] {
  accent-color: #89b4fa;
}

.dialog-buttons {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}

.btn {
  padding: 6px 20px;
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
