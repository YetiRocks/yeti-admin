import { createRootRoute, Outlet } from '@tanstack/react-router'
import { useToast } from '../hooks/useToast'

export const Route = createRootRoute({
  component: RootLayout,
})

function RootLayout() {
  const { ToastContainer } = useToast()

  return (
    <div className="app-shell">
      <header className="header">
        <div className="header-left">
          <img src="logo_white.svg" alt="Yeti" className="logo" />
        </div>
        <div className="header-right">
          <div className="header-title">Application Manager</div>
        </div>
      </header>

      <div className="main-content">
        <Outlet />
      </div>

      <ToastContainer />
    </div>
  )
}
