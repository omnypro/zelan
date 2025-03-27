type StatusIndicatorProps = {
  status: 'connected' | 'disconnected' | 'error' | 'disabled'
}

export function StatusIndicator({ status }: StatusIndicatorProps) {
  let bgColor = 'bg-gray-400'
  let textColor = 'text-gray-700'
  
  switch (status) {
    case 'connected':
      bgColor = 'bg-green-500'
      textColor = 'text-green-700'
      break
    case 'disconnected':
      bgColor = 'bg-yellow-400'
      textColor = 'text-yellow-700'
      break
    case 'error':
      bgColor = 'bg-red-500'
      textColor = 'text-red-700'
      break
    case 'disabled':
      bgColor = 'bg-gray-300'
      textColor = 'text-gray-500'
      break
  }
  
  return (
    <div className={`flex items-center gap-1 text-xs font-medium px-2 py-0.5 rounded-full ${bgColor} ${textColor}`}>
      <div className={`w-1.5 h-1.5 rounded-full ${bgColor.replace('bg-', 'bg-')}`}></div>
      <span className="capitalize">{status}</span>
    </div>
  )
}