export const BASE = '/admin'
export const AUTH_BASE = '/yeti-auth'
export const TELEMETRY_BASE = '/yeti-telemetry'
export const VECTORS_BASE = '/yeti-vectors'

// Session expiry callback — set by the root layout to transition to login screen
let onSessionExpired: (() => void) | null = null

export function setSessionExpiredHandler(handler: () => void) {
  onSessionExpired = handler
}

// Debounce: only handle the first 401, ignore subsequent ones from parallel requests
let sessionExpiring = false

export async function api<T = unknown>(url: string, options: RequestInit = {}): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string> || {}),
  }

  const res = await fetch(url, { ...options, headers, credentials: 'same-origin' })
  if (res.status === 401) {
    if (!sessionExpiring) {
      sessionExpiring = true
      if (onSessionExpired) {
        onSessionExpired()
      }
      // Reset after a short delay so future 401s (after re-login) are handled
      setTimeout(() => { sessionExpiring = false }, 1000)
    }
    // Return a never-resolving promise so the caller suspends silently
    // while the session-expired handler transitions to the login page.
    // Throwing here would crash route components before React re-renders.
    return new Promise<T>(() => {})
  }
  if (!res.ok) {
    const text = await res.text()
    throw new Error(text || res.statusText)
  }
  const text = await res.text()
  return text ? JSON.parse(text) : (null as T)
}
