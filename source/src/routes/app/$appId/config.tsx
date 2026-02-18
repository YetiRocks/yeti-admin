import { createFileRoute, getRouteApi } from '@tanstack/react-router'

const parentRoute = getRouteApi('/app/$appId')

export const Route = createFileRoute('/app/$appId/config')({
  component: ConfigView,
})

function ConfigView() {
  const { detail } = parentRoute.useLoaderData()
  const config = detail?.config

  return (
    <div className="config-view">
      <textarea
        className="config-textarea full"
        readOnly
        value={config ? JSON.stringify(config, null, 2) : 'No config available'}
      />
    </div>
  )
}
