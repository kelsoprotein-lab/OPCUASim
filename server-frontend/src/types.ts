export interface ServerNodeInfo {
  node_id: string
  display_name: string
  parent_id: string
  data_type: string
  writable: boolean
  simulation: SimulationMode
  current_value?: string
}

export interface ServerFolderInfo {
  node_id: string
  display_name: string
  parent_id: string
}

export type SimulationMode =
  | { type: 'Static'; value: string }
  | { type: 'Random'; min: number; max: number; interval_ms: number }
  | { type: 'Sine'; amplitude: number; offset: number; period_ms: number; interval_ms: number }
  | { type: 'Linear'; start: number; step: number; min: number; max: number; mode: 'Repeat' | 'Bounce'; interval_ms: number }
  | { type: 'Script'; expression: string; interval_ms: number }

export interface ServerStatus {
  state: string
  node_count: number
  folder_count: number
}

export interface ServerConfig {
  name: string
  endpoint_url: string
  port: number
  security_policies: string[]
  security_modes: string[]
  users: UserAccount[]
  anonymous_enabled: boolean
  max_sessions: number
  max_subscriptions_per_session: number
}

export interface UserAccount {
  username: string
  password: string
  role: 'ReadOnly' | 'ReadWrite' | 'Admin'
}

export interface AddressSpace {
  folders: ServerFolderInfo[]
  nodes: ServerNodeInfo[]
}
