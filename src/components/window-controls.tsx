import { useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { cn } from '@/lib/utils'

type ButtonType = 'close' | 'minimize' | 'maximize'

export const MacOSWindowControls = () => {
  const renderButton = (type: ButtonType): JSX.Element => {
    return (
      <div
        className={cn(
          'size-3 cursor-pointer rounded-full flex items-center justify-center text-xs font-medium text-white transition-colors duration-200 ease-in-out',
          { 'bg-[#ff5f57]': type === 'close' },
          { 'bg-[#ffbd2e]': type === 'minimize' },
          { 'bg-[#28c941]': type === 'maximize' },
        )}>
        
      </div>
    )
  }

  return (
    <div className="flex h-10 items-center gap-2">
      {renderButton('close')}
      {renderButton('minimize')}
      {renderButton('maximize')}
    </div>
  )
}

const MacOSTrafficLights = () => {
  // State to track hover status
  const [hoveredButton, setHoveredButton] = useState(null)

  // Define the official colors for each button
  const buttonColors = {
    close: {
      base: '#FF5F57',
      hover: '#FF5F57',
      symbol: '#4D0000'
    },
    minimize: {
      base: '#FFBD2E',
      hover: '#FFBD2E',
      symbol: '#6A5100'
    },
    maximize: {
      base: '#28C941',
      hover: '#28C941',
      symbol: '#0A5700'
    }
  }

  // Common styles for the buttons
  const buttonStyle = {
    width: '12px',
    height: '12px',
    borderRadius: '50%',
    marginRight: '8px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
    transition: 'all 0.1s ease'
  }

  // Symbol styles (only visible on hover)
  const symbolStyle = {
    fontSize: '9px',
    fontWeight: 'bold',
    lineHeight: 1,
    visibility: 'hidden',
    userSelect: 'none'
  }

  // Function to render a single button
  const renderButton = (type) => {
    const isHovered = hoveredButton === type

    return (
      <div
        style={{
          ...buttonStyle,
          backgroundColor: buttonColors[type].base,
          border: `0.5px solid ${isHovered ? buttonColors[type].symbol : 'rgba(0, 0, 0, 0.1)'}`
        }}
        onMouseEnter={() => setHoveredButton(type)}
        onMouseLeave={() => setHoveredButton(null)}>
        <span
          style={{
            ...symbolStyle,
            color: buttonColors[type].symbol,
            visibility: isHovered ? 'visible' : 'hidden'
          }}>
          {type === 'close' && '×'}
          {type === 'minimize' && '−'}
          {type === 'maximize' && '+'}
        </span>
      </div>
    )
  }

  return (
    <div style={{ padding: '20px' }}>
      <div
        style={{
          height: '38px',
          backgroundColor: '#E5E5E5',
          borderRadius: '6px 6px 0 0',
          display: 'flex',
          alignItems: 'center',
          paddingLeft: '12px'
        }}>
        <div
          style={{
            display: 'flex',
            alignItems: 'center'
          }}>
          {renderButton('close')}
          {renderButton('minimize')}
          {renderButton('maximize')}
        </div>
        <div
          style={{
            flex: 1,
            textAlign: 'center',
            fontSize: '13px',
            color: '#4D4D4D',
            fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
            fontWeight: 500
          }}>
          Window Title
        </div>
      </div>
      <div
        style={{
          height: '200px',
          backgroundColor: '#F5F5F5',
          borderRadius: '0 0 6px 6px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: '#8E8E8E',
          fontSize: '16px',
          fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif'
        }}>
        Window Content
      </div>
    </div>
  )
}

export default MacOSTrafficLights
