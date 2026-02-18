export interface AppSummary {
  app_id: string
  name: string
  enabled: boolean
  has_schema: boolean
  resource_count: number
  is_extension: boolean
}

export interface AppDetail {
  app_id: string
  config: AppConfig | null
  files: string[]
  has_schema: boolean
  resource_count: number
}

export interface AppConfig {
  name?: string
  app_id?: string
  version?: string
  description?: string
  enabled?: boolean
  extension?: boolean
  extensions?: Array<string | Record<string, unknown>>
  [key: string]: unknown
}

export interface SchemaInfo {
  tables: TableInfo[]
}

export interface TableInfo {
  name: string
  database: string
  rest_url: string
  fields?: { name: string; type: string }[]
}

export interface PaginatedResponse {
  data: Record<string, unknown>[]
  total: number
  limit: number
  offset: number
}

export interface FileEntry {
  name: string
  type: 'file' | 'directory'
  size: number
}

export interface GitStatus {
  app_id: string
  is_git: boolean
  branch?: string
  remote_url?: string
  dirty?: boolean
}

export interface DeployKey {
  name: string
  public_key: string
  created?: number
}

export interface AuthUser {
  username: string
  email?: string
  roleId?: string
  active?: boolean
}

export interface AuthRole {
  id: string
  name?: string
  permissions?: { super_user?: boolean }
}

export interface RepoCheck {
  url: string
  public: boolean
}

export type Tab = 'overview' | 'files' | 'data' | 'auth' | 'keys'
