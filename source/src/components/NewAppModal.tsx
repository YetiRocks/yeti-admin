import { useState } from 'react'

type Mode = null | 'yetirocks' | 'public' | 'template' | 'private'

interface Props {
  installedApps: Set<string>
  onClose: () => void
}

function sanitizeAppName(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9\-]/g, '')
}

export function NewAppModal({ installedApps, onClose }: Props) {
  const [mode, setMode] = useState<Mode>(null)
  const [url, setUrl] = useState('')
  const [appName, setAppName] = useState('')

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={e => e.stopPropagation()}>
        <div className="modal-title">New Application</div>

        {mode === null && (
          <div className="modal-options">
            <button className="modal-option" onClick={() => setMode('yetirocks')}>
              <span className="modal-option-name">Clone from Yeti Rocks</span>
              <span className="modal-option-desc">Browse public application repos from github.com/yetirocks</span>
            </button>
            <button className="modal-option" onClick={() => setMode('public')}>
              <span className="modal-option-name">Clone Public Repo</span>
              <span className="modal-option-desc">Clone any public git repository by URL</span>
            </button>
            <button className="modal-option" onClick={() => setMode('template')}>
              <span className="modal-option-name">Blank Template</span>
              <span className="modal-option-desc">Start with an empty application template</span>
            </button>
            <button className="modal-option" onClick={() => setMode('private')}>
              <span className="modal-option-name">Clone Private Repo</span>
              <span className="modal-option-desc">Requires a deployment certificate</span>
            </button>
          </div>
        )}

        {mode === 'yetirocks' && (
          <YetiRocksView installedApps={installedApps} onBack={() => setMode(null)} />
        )}

        {mode === 'public' && (
          <div className="modal-form">
            <label className="modal-label">Repository URL</label>
            <input
              className="filter-input"
              style={{ maxWidth: 'none' }}
              placeholder="https://github.com/user/repo.git"
              value={url}
              onChange={e => setUrl(e.target.value)}
              autoFocus
            />
            <div className="modal-actions">
              <button className="btn" onClick={() => { setMode(null); setUrl('') }}>Back</button>
              <button className="btn btn-primary" disabled={!url.trim()}>Clone</button>
            </div>
          </div>
        )}

        {mode === 'template' && (
          <div className="modal-form">
            <label className="modal-label">Application Name</label>
            <input
              className="filter-input"
              style={{ maxWidth: 'none' }}
              placeholder="my-app"
              value={appName}
              onChange={e => setAppName(sanitizeAppName(e.target.value))}
              autoFocus
            />
            <p className="modal-form-hint">
              Lowercase letters, numbers, and hyphens only.
            </p>
            <div className="modal-actions">
              <button className="btn" onClick={() => { setMode(null); setAppName('') }}>Back</button>
              <button className="btn btn-primary" disabled={!appName}>Create</button>
            </div>
          </div>
        )}

        {mode === 'private' && (
          <div className="modal-form">
            <label className="modal-label">Repository URL</label>
            <input
              className="filter-input"
              style={{ maxWidth: 'none' }}
              placeholder="git@github.com:org/repo.git"
              value={url}
              onChange={e => setUrl(e.target.value)}
              autoFocus
            />
            <p className="modal-form-hint">
              Requires a deployment certificate configured on your server.
            </p>
            <div className="modal-actions">
              <button className="btn" onClick={() => { setMode(null); setUrl('') }}>Back</button>
              <button className="btn btn-primary" disabled={!url.trim()}>Clone</button>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

function YetiRocksView({ installedApps, onBack }: { installedApps: Set<string>; onBack: () => void }) {
  const [loading, setLoading] = useState(true)
  const [repos, setRepos] = useState<{ name: string; description: string; html_url: string }[]>([])
  const [error, setError] = useState('')

  useState(() => {
    fetch('https://api.github.com/orgs/yetirocks/repos?type=public&per_page=100&sort=name')
      .then(r => {
        if (!r.ok) throw new Error(`GitHub API: ${r.status}`)
        return r.json()
      })
      .then((data: { name: string; description: string; html_url: string }[]) => {
        setRepos(data.filter(r => r.name !== '.github' && r.name !== 'yeti' && !r.name.startsWith('yeti-')))
        setLoading(false)
      })
      .catch(e => {
        setError(String(e))
        setLoading(false)
      })
  })

  return (
    <div className="modal-form">
      {loading && <p className="modal-form-text">Loading repos...</p>}
      {error && <p className="modal-error">{error}</p>}
      {!loading && !error && repos.length === 0 && (
        <p className="modal-form-text">No public repos found.</p>
      )}
      {!loading && !error && repos.length > 0 && (
        <div className="modal-repo-list">
          {repos.map(repo => {
            const installed = installedApps.has(repo.name)
            return (
              <button
                key={repo.name}
                className={`modal-option${installed ? ' modal-option-disabled' : ''}`}
                disabled={installed}
              >
                <span className="modal-option-row">
                  <span className="modal-option-name">{repo.name}</span>
                  {installed && <span className="installed-badge">installed</span>}
                </span>
                {repo.description && <span className="modal-option-desc">{repo.description}</span>}
              </button>
            )
          })}
        </div>
      )}
      <div className="modal-actions">
        <button className="btn" onClick={onBack}>Back</button>
      </div>
    </div>
  )
}
