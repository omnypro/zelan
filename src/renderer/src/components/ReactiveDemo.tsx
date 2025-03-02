import { useState, useEffect } from 'react'
import { createStateContainer } from '@shared/core/utils/stateManagement'
import { createDataStream } from '@shared/core/utils/dataStream'
import { useStateContainer, useDataStream } from '@renderer/hooks/useReactiveState'
import { createAsyncOperation, AsyncStatus } from '@shared/core/utils/asyncOperations'
import { BehaviorSubject, interval } from 'rxjs'
import { map, take } from 'rxjs/operators'

// Create a state container for the application state
interface AppState {
  counter: number
  lastUpdated: number
  notes: string[]
}

const appStateContainer = createStateContainer<AppState>({
  counter: 0,
  lastUpdated: Date.now(),
  notes: ['Initial note']
})

// Create a data stream for a counter
const counterStream = createDataStream<number>({
  initialValue: 0,
  name: 'CounterStream',
  debug: true
})

// Create a mock async operation
const fetchRandomNumber = createAsyncOperation<number>(
  async () => {
    // Simulate network delay
    await new Promise((resolve) => setTimeout(resolve, 1000))
    return Math.floor(Math.random() * 100)
  },
  {
    name: 'FetchRandomNumber',
    retry: true,
    retryCount: 3,
    debug: true
  }
)

// Demo component for reactive features
export function ReactiveDemo() {
  // Use the state container
  const [appState, updateAppState] = useStateContainer(appStateContainer)

  // Use the data stream
  const [counterValue, setCounterValue] = useDataStream(counterStream)

  // State for async operation
  const [randomNumber, setRandomNumber] = useState<number | null>(null)
  const [loadingStatus, setLoadingStatus] = useState<AsyncStatus>(AsyncStatus.IDLE)

  // Increment counter in state container
  const incrementCounter = () => {
    updateAppState({
      counter: appState.counter + 1,
      lastUpdated: Date.now()
    })
  }

  // Add a note to state container
  const addNote = () => {
    const newNote = `Note added at ${new Date().toLocaleTimeString()}`
    updateAppState({
      notes: [...appState.notes, newNote]
    })
  }

  // Increment counter in data stream
  const incrementStreamCounter = () => {
    if (counterValue !== undefined) {
      setCounterValue(counterValue + 1)
    }
  }

  // Fetch a random number
  const fetchRandom = () => {
    setLoadingStatus(AsyncStatus.PENDING)

    fetchRandomNumber.execute(undefined).subscribe({
      next: (value) => {
        setRandomNumber(value)
        setLoadingStatus(AsyncStatus.SUCCESS)
      },
      error: (error) => {
        console.error('Error fetching random number:', error)
        setLoadingStatus(AsyncStatus.ERROR)
      }
    })
  }

  // Create an interval counter that updates every 2 seconds
  useEffect(() => {
    const subscription = interval(2000)
      .pipe(
        take(10),
        map((count) => count + 1)
      )
      .subscribe((count) => {
        // Update a different data stream on an interval
        counterStream.push((counterValue ?? 0) + 0.5)
      })

    return () => subscription.unsubscribe()
  }, [counterValue])

  return (
    <div className="reactive-demo">
      <h2>Reactive State Management Demo</h2>

      <div className="demo-section">
        <h3>State Container Demo</h3>
        <p>
          Counter value: <strong>{appState.counter}</strong>
        </p>
        <p>
          Last updated: <strong>{new Date(appState.lastUpdated).toLocaleTimeString()}</strong>
        </p>
        <button onClick={incrementCounter}>Increment Counter</button>
        <button onClick={addNote}>Add Note</button>

        <div className="notes-section">
          <h4>Notes:</h4>
          <ul>
            {appState.notes.map((note, index) => (
              <li key={index}>{note}</li>
            ))}
          </ul>
        </div>
      </div>

      <div className="demo-section">
        <h3>Data Stream Demo</h3>
        <p>
          Stream counter: <strong>{counterValue ?? 0}</strong>
        </p>
        <p>
          <em>Note: Stream counter auto-increments by 0.5 every 2 seconds</em>
        </p>
        <button onClick={incrementStreamCounter}>Increment Stream Counter</button>
      </div>

      <div className="demo-section">
        <h3>Async Operation Demo</h3>
        <p>
          Random number: <strong>{randomNumber !== null ? randomNumber : 'Not fetched yet'}</strong>
        </p>
        <p>
          Status: <strong>{loadingStatus}</strong>
        </p>
        <button onClick={fetchRandom} disabled={loadingStatus === AsyncStatus.PENDING}>
          {loadingStatus === AsyncStatus.PENDING ? 'Loading...' : 'Fetch Random Number'}
        </button>
      </div>
    </div>
  )
}

export default ReactiveDemo
