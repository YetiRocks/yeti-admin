import {
  useReactTable,
  getCoreRowModel,
  flexRender,
  createColumnHelper,
  type ColumnDef,
} from '@tanstack/react-table'
import { useMemo, useState } from 'react'

interface Props {
  records: Record<string, unknown>[]
  fields: { name: string; type: string }[]
  total: number
  page: number
  pageSize: number
  onPageChange: (page: number) => void
  onUpdate: (record: Record<string, unknown>) => Promise<void>
}

function formatCell(val: unknown): string {
  if (val === null || val === undefined) return ''
  if (typeof val === 'object') return JSON.stringify(val)
  return String(val)
}

export function DataTable({ records, fields, total, page, pageSize, onPageChange, onUpdate }: Props) {
  const [selected, setSelected] = useState<Record<string, unknown> | null>(null)
  const [editJson, setEditJson] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')

  const columns = useMemo<ColumnDef<Record<string, unknown>, unknown>[]>(() => {
    const keys = fields.length > 0
      ? fields.map(f => f.name)
      : [...new Set(records.flatMap(r => Object.keys(r)))]
    const helper = createColumnHelper<Record<string, unknown>>()
    return keys.map(key =>
      helper.accessor(row => row[key], {
        id: key,
        header: key,
        cell: info => {
          const full = formatCell(info.getValue())
          const truncated = full.length > 80 ? full.substring(0, 80) + '...' : full
          return <span title={full}>{truncated}</span>
        },
      })
    )
  }, [fields, records])

  const table = useReactTable({
    data: records,
    columns,
    getCoreRowModel: getCoreRowModel(),
  })

  function openDetail(record: Record<string, unknown>) {
    setSelected(record)
    setEditJson(JSON.stringify(record, null, 2))
    setError('')
  }

  function closeDetail() {
    setSelected(null)
    setEditJson('')
    setError('')
  }

  async function handleUpdate() {
    let parsed: Record<string, unknown>
    try {
      parsed = JSON.parse(editJson)
    } catch {
      setError('Invalid JSON')
      return
    }
    setSaving(true)
    setError('')
    try {
      await onUpdate(parsed)
      closeDetail()
    } catch (e) {
      setError(String(e))
    } finally {
      setSaving(false)
    }
  }

  if (total === 0) {
    return <div className="empty-state">No records</div>
  }

  const pageCount = Math.max(1, Math.ceil(total / pageSize))

  return (
    <div className="data-table-wrapper">
      <div style={{ overflowX: 'auto' }}>
        <table className="data-table">
          <thead>
            {table.getHeaderGroups().map(hg => (
              <tr key={hg.id}>
                {hg.headers.map(header => (
                  <th key={header.id}>
                    {flexRender(header.column.columnDef.header, header.getContext())}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map(row => (
              <tr
                key={row.id}
                className="data-table-row-clickable"
                onClick={() => openDetail(row.original)}
              >
                {row.getVisibleCells().map(cell => (
                  <td key={cell.id}>
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="pagination-controls">
        <button
          className="btn"
          onClick={() => onPageChange(page - 1)}
          disabled={page <= 1}
        >
          Previous
        </button>
        <span className="pagination-info">
          Page {page} of {pageCount}
        </span>
        <button
          className="btn"
          onClick={() => onPageChange(page + 1)}
          disabled={page >= pageCount}
        >
          Next
        </button>
        <span className="pagination-count">
          {total} record{total !== 1 ? 's' : ''}
        </span>
      </div>

      {selected && (
        <div className="modal-overlay" onClick={closeDetail}>
          <div className="modal-content" onClick={e => e.stopPropagation()}>
            <textarea
              className="modal-textarea"
              value={editJson}
              onChange={e => setEditJson(e.target.value)}
              spellCheck={false}
            />
            {error && <div className="modal-error">{error}</div>}
            <div className="modal-actions">
              <button className="btn" onClick={closeDetail}>Cancel</button>
              <button className="btn btn-primary" onClick={handleUpdate} disabled={saving}>
                {saving ? 'Updating...' : 'Update'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
