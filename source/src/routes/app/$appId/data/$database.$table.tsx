import { createFileRoute, getRouteApi } from '@tanstack/react-router'
import { useState, useEffect, useCallback } from 'react'
import { api } from '../../../../api'
import { PaginatedResponse } from '../../../../types'
import { DataTable } from '../../../../components/DataTable'

const PAGE_SIZE = 25
const parentRoute = getRouteApi('/app/$appId')

export const Route = createFileRoute('/app/$appId/data/$database/$table')({
  component: DataView,
})

function DataView() {
  const { appId, table } = Route.useParams()
  const { schema } = parentRoute.useLoaderData()

  const [page, setPage] = useState(1)
  const [data, setData] = useState<PaginatedResponse | null>(null)
  const [loading, setLoading] = useState(true)

  const tableInfo = schema?.tables.find(t => t.name === table)
  const fields = tableInfo?.fields || []

  const fetchPage = useCallback((p: number) => {
    setLoading(true)
    const offset = (p - 1) * PAGE_SIZE
    api<PaginatedResponse>(`/${appId}/${table}/?pagination=true&limit=${PAGE_SIZE}&offset=${offset}`)
      .then(res => setData(res))
      .catch(() => setData({ data: [], total: 0, limit: PAGE_SIZE, offset: 0 }))
      .finally(() => setLoading(false))
  }, [appId, table])

  // Reset page when table changes
  useEffect(() => { setPage(1) }, [appId, table])

  useEffect(() => { fetchPage(page) }, [fetchPage, page])

  const handleUpdate = useCallback(async (record: Record<string, unknown>) => {
    const id = record.id
    if (id == null) throw new Error('Record has no id field')
    await api(`/${appId}/${table}/${encodeURIComponent(String(id))}`, {
      method: 'PUT',
      body: JSON.stringify(record),
    })
    fetchPage(page)
  }, [appId, table, fetchPage, page])

  if (loading && !data) {
    return <div className="empty-state">Loading...</div>
  }

  const records = data?.data || []
  const total = data?.total || 0

  return (
    <DataTable
      records={records}
      fields={fields}
      total={total}
      page={page}
      pageSize={PAGE_SIZE}
      onPageChange={setPage}
      onUpdate={handleUpdate}
    />
  )
}
