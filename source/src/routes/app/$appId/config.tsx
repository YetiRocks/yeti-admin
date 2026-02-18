import { createFileRoute, getRouteApi } from '@tanstack/react-router'
import { useMemo } from 'react'

const parentRoute = getRouteApi('/app/$appId')

export const Route = createFileRoute('/app/$appId/config')({
  component: ConfigView,
})

function highlightJson(json: string): string {
  return json.replace(
    /("(?:\\.|[^"\\])*")\s*:|("(?:\\.|[^"\\])*")|(true|false|null)|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g,
    (match, key, str, bool, num) => {
      if (key) return `<span class="json-key">${key}</span>:`
      if (str) return `<span class="json-string">${str}</span>`
      if (bool) return `<span class="json-bool">${bool}</span>`
      if (num) return `<span class="json-number">${num}</span>`
      return match
    }
  )
}

function ConfigView() {
  const { detail } = parentRoute.useLoaderData()
  const config = detail?.config

  const highlighted = useMemo(() => {
    if (!config) return null
    return highlightJson(JSON.stringify(config, null, 2))
  }, [config])

  return (
    <div className="config-view">
      {highlighted ? (
        <pre className="config-code full"><code dangerouslySetInnerHTML={{ __html: highlighted }} /></pre>
      ) : (
        <pre className="config-code full"><code>No config available</code></pre>
      )}
    </div>
  )
}
