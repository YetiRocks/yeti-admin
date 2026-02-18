const BASE = '/yeti-applications'
const AUTH_BASE = '/yeti-auth'

export { BASE, AUTH_BASE }

export async function api<T = unknown>(url: string, options: RequestInit = {}): Promise<T> {
  const res = await fetch(url, {
    headers: { 'Content-Type': 'application/json', ...options.headers },
    ...options,
  })
  if (!res.ok) {
    const text = await res.text()
    throw new Error(text || res.statusText)
  }
  const text = await res.text()
  return text ? JSON.parse(text) : (null as T)
}
