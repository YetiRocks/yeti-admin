import { useState, useCallback } from 'react'

interface Toast {
  id: number
  message: string
  type: 'info' | 'success' | 'error'
}

let nextId = 0

export function useToast() {
  const [toasts, setToasts] = useState<Toast[]>([])

  const toast = useCallback((message: string, type: 'info' | 'success' | 'error' = 'info') => {
    const id = nextId++
    setToasts(prev => [...prev, { id, message, type }])
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id))
    }, 3000)
  }, [])

  const ToastContainer = () => (
    <>
      {toasts.map(t => (
        <div key={t.id} className={`toast ${t.type}`}>{t.message}</div>
      ))}
    </>
  )

  return { toast, ToastContainer }
}
