import { createRootRoute, Outlet, Link, useRouter } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import { useToast } from '../hooks/useToast'
import { BASE, AUTH_BASE, setSessionExpiredHandler } from '../api'

export const Route = createRootRoute({
  component: RootLayout,
})

function LoginPage({ onLogin }: { onLogin: () => void }) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')
    setLoading(true)
    try {
      const res = await fetch(`${AUTH_BASE}/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username, password }),
        credentials: 'same-origin',
      })
      if (!res.ok) {
        const text = await res.text()
        throw new Error(text || 'Login failed')
      }
      // Cookie is set by the server response (httpOnly)
      onLogin()
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Login failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="login-page">
      <div className="login-card">
        <img src={`${import.meta.env.BASE_URL}logo_white.svg`} alt="Yeti" className="login-logo" />
        <form onSubmit={handleSubmit}>
          <input
            type="text"
            placeholder="Username"
            value={username}
            onChange={e => setUsername(e.target.value)}
            autoFocus
          />
          <input
            type="password"
            placeholder="Password"
            value={password}
            onChange={e => setPassword(e.target.value)}
          />
          {error && <div className="login-error">{error}</div>}
          <button type="submit" disabled={loading || !username || !password}>
            {loading ? 'Signing in...' : 'Sign In'}
          </button>
        </form>
      </div>
    </div>
  )
}

function RootLayout() {
  const [authenticated, setAuthenticated] = useState<boolean | null>(null)
  const { ToastContainer } = useToast()
  const router = useRouter()

  const checkAuth = useCallback(async () => {
    try {
      // Hit an admin endpoint that goes through the auth pipeline.
      // /yeti-auth/auth doesn't work for JWT (yeti-auth's own router
      // doesn't include itself as an auth extension).
      const res = await fetch(`${BASE}/appvalidation/`, { credentials: 'same-origin' })
      setAuthenticated(res.ok)
    } catch {
      setAuthenticated(false)
    }
  }, [])

  useEffect(() => {
    checkAuth()
  }, [checkAuth])

  // Register the 401 handler so api() calls transition to login without reloading
  useEffect(() => {
    setSessionExpiredHandler(() => setAuthenticated(false))
  }, [])

  const handleLogin = () => {
    // Small delay to let the browser store the httpOnly cookie from the
    // login response before firing authenticated API calls
    setTimeout(() => setAuthenticated(true), 50)
  }

  const handleLogout = async () => {
    try {
      await fetch(`${AUTH_BASE}/login`, {
        method: 'DELETE',
        credentials: 'same-origin',
      })
    } catch {
      // Best-effort logout
    }
    setAuthenticated(false)
    router.navigate({ to: '/' })
  }

  if (authenticated === null) {
    return <div className="loading">Loading...</div>
  }

  if (!authenticated) {
    return <LoginPage onLogin={handleLogin} />
  }

  return (
    <div className="app">
      <nav className="nav">
        <div className="nav-left">
          <a href="/">
            <img src={`${import.meta.env.BASE_URL}logo_white.svg`} alt="Yeti" className="nav-logo" />
          </a>
        </div>
        <div className="nav-center">
          <Link to="/applications" className="nav-link" activeProps={{ className: 'nav-link active' }}>
            Applications
          </Link>
          <Link to="/auth" className="nav-link" activeProps={{ className: 'nav-link active' }}>
            Auth
          </Link>
          <Link to="/telemetry" className="nav-link" activeProps={{ className: 'nav-link active' }}>
            Telemetry
          </Link>
          <Link to="/vectors" className="nav-link" activeProps={{ className: 'nav-link active' }}>
            Vectors
          </Link>
          <Link to="/benchmarks" className="nav-link" activeProps={{ className: 'nav-link active' }}>
            Benchmarks
          </Link>
        </div>
        <div className="nav-right">
          <button className="btn nav-action-btn" onClick={handleLogout}>Log Out</button>
        </div>
      </nav>
      <div className="page">
        <div className="admin-layout">
          <Outlet />
        </div>
      </div>

      <ToastContainer />
    </div>
  )
}
