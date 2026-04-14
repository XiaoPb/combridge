import { decode } from '@msgpack/msgpack';
import type { EventBusEvent } from '../api/events';

export type { EventBusEvent };

export function decodePayload<T>(event: EventBusEvent): T {
  if (event.encoding === 'json') {
    return JSON.parse(event.payload) as T;
  }

  if (event.encoding === 'msgpack+base64') {
    const binaryString = atob(event.payload);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return decode(bytes) as T;
  }

  throw new Error(`Unknown encoding: ${event.encoding}`);
}
