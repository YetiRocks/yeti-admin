import { AppSummary } from '../types'

export function AppCard({ app, onClick }: { app: AppSummary; onClick: () => void }) {
  const status = app.enabled ? 'running' : 'disabled'

  return (
    <div className={`app-card ${status}`} onClick={onClick}>
      <div className="app-card-top">
        <span className={`status-dot ${status}`} />
        <span className="status-label">{app.enabled ? 'Running' : 'Disabled'}</span>
        {app.is_extension && <span className="ext-badge">ext</span>}
      </div>
      <div className="app-card-name">{app.app_id}</div>
      <div className="app-card-stats">
        {app.has_schema && <span>{app.table_count} table{app.table_count !== 1 ? 's' : ''}</span>}
      </div>
    </div>
  )
}
