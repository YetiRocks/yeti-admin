import { createFileRoute, Outlet, Link, useLocation } from '@tanstack/react-router'
import { api, BASE } from '../../../api'
import { AppDetail, SchemaInfo, PaginatedResponse } from '../../../types'
import { DatabaseNav, groupTablesByDatabase } from '../../../components/DatabaseNav'

export const Route = createFileRoute('/app/$appId')({
  loader: async ({ params }) => {
    const [detail, schema] = await Promise.all([
      api<AppDetail>(`${BASE}/apps/${params.appId}`),
      api<SchemaInfo>(`${BASE}/schemas/${params.appId}`),
    ])

    // Fetch record counts for all tables in parallel
    const tables = schema?.tables || []
    const countEntries = await Promise.all(
      tables.map(t =>
        api<PaginatedResponse>(`${t.rest_url}/?pagination=true&limit=0`)
          .then(r => [t.name, r.total] as const)
          .catch(() => [t.name, 0] as const)
      )
    )
    const counts: Record<string, number> = Object.fromEntries(countEntries)

    return { detail, schema, counts }
  },
  component: AppLayout,
})

function AppLayout() {
  const { appId } = Route.useParams()
  const { detail, schema, counts } = Route.useLoaderData()
  const location = useLocation()

  const config = detail?.config
  const enabled = config?.enabled !== false
  const tables = schema?.tables || []

  const isConfig = location.pathname.endsWith('/config')
  const isData = location.pathname.includes('/data/')

  // Find first db/table for the Data link
  const groups = groupTablesByDatabase(tables, appId)
  const firstEntry = groups.entries().next().value
  const firstDb = firstEntry ? firstEntry[0] : null
  const firstTable = firstEntry ? firstEntry[1][0]?.name : null

  return (
    <div className="detail-view">
      <div className="detail-header">
        <div className="detail-header-left">
          <Link to="/" className="back-btn">&lt; All Apps</Link>
        </div>
        <span className="detail-app-name">{appId}</span>
        <span className={`status-dot ${enabled ? 'running' : 'disabled'}`} />
        <span className={`status-label ${enabled ? 'running' : 'disabled'}`}>
          {enabled ? 'Running' : 'Disabled'}
        </span>
        {config?.extension && <span className="ext-badge">ext</span>}

        <div className="subnav">
          {firstDb && firstTable && (
            <Link
              to="/app/$appId/data/$database/$table"
              params={{ appId, database: firstDb, table: firstTable }}
              className={`subnav-link ${isData ? 'active' : ''}`}
            >
              Data
            </Link>
          )}
          <Link
            to="/app/$appId/config"
            params={{ appId }}
            className={`subnav-link ${isConfig ? 'active' : ''}`}
          >
            Config
          </Link>
        </div>
      </div>

      <div className="detail-body-layout">
        {isData && tables.length > 0
          ? <DatabaseNav appId={appId} tables={tables} counts={counts} fallbackDb={appId} />
          : <div className="db-nav" />
        }
        <div className="detail-content">
          <Outlet />
        </div>
      </div>
    </div>
  )
}
