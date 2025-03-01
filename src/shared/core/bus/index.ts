export * from './EventBus'
export * from './createEventStream'

import { EventBus } from './EventBus'
import { createEventStream, createEventClassStream } from './createEventStream'

export default {
  EventBus,
  createEventStream,
  createEventClassStream
}
