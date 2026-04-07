export interface ConnectionInfo {
  id: string
  name: string
  endpoint_url: string
  security_policy: string
  security_mode: string
  auth_type: string
  state: string
}

export interface BrowseResult {
  node_id: string
  display_name: string
  node_class: string
  data_type?: string
  has_children: boolean
}

export interface MonitoredNodeInfo {
  node_id: string
  display_name: string
  browse_path: string
  data_type: string
  value?: string
  quality?: string
  timestamp?: string
  access_mode: string
  interval_ms: number
  group_id?: string
}

export interface NodeGroupInfo {
  id: string
  name: string
  node_count: number
}

export interface NodeAttributesInfo {
  node_id: string
  display_name: string
  description: string
  data_type: string
  access_level: string
  value?: string
  quality?: string
  timestamp?: string
}

export interface ConnectionStateEvent {
  id: string
  state: string
}

export interface DataChangedEvent {
  connection_id: string
  items: DataChangeItem[]
}

export interface DataChangeItem {
  node_id: string
  value: string
  quality: string
  timestamp: string
}
