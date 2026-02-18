import { createFileRoute, redirect } from '@tanstack/react-router'
import { api, BASE } from '../../../api'
import { SchemaInfo } from '../../../types'
import { groupTablesByDatabase } from '../../../components/DatabaseNav'

export const Route = createFileRoute('/app/$appId/')({
  beforeLoad: async ({ params }) => {
    const schema = await api<SchemaInfo>(`${BASE}/schemas/${params.appId}`)
    const tables = schema?.tables || []

    if (tables.length > 0) {
      const groups = groupTablesByDatabase(tables, params.appId)
      const firstEntry = groups.entries().next().value
      if (firstEntry) {
        const [db, tbls] = firstEntry
        throw redirect({
          to: '/app/$appId/data/$database/$table',
          params: { appId: params.appId, database: db, table: tbls[0].name },
        })
      }
    }

    // No tables — redirect to config
    throw redirect({
      to: '/app/$appId/config',
      params: { appId: params.appId },
    })
  },
})
