import { useState } from 'react'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { api, BASE } from '../api'
import { AppSummary } from '../types'
import { AppCard } from '../components/AppCard'

export const Route = createFileRoute('/')({
  loader: () => api<AppSummary[]>(`${BASE}/apps`),
  component: CardGrid,
})

function CardGrid() {
  const apps = Route.useLoaderData()
  const navigate = useNavigate()
  const [filter, setFilter] = useState('')

  const sorted = [...apps].sort((a, b) => {
    if (a.is_extension !== b.is_extension) return a.is_extension ? -1 : 1
    return a.app_id.localeCompare(b.app_id)
  })

  const filtered = filter
    ? sorted.filter(app => app.app_id.toLowerCase().includes(filter.toLowerCase()))
    : sorted

  return (
    <div className="home-view">
      <div className="home-toolbar">
        <input
          type="text"
          className="filter-input"
          placeholder="Filter applications..."
          value={filter}
          onChange={e => setFilter(e.target.value)}
        />
        <button className="btn btn-primary" style={{ marginLeft: 'auto' }} onClick={() => {}}>
          New Application
        </button>
      </div>
      <div className="card-grid">
        {filtered.map(app => (
          <AppCard
            key={app.app_id}
            app={app}
            onClick={() => navigate({ to: '/app/$appId', params: { appId: app.app_id } })}
          />
        ))}
      </div>
    </div>
  )
}
