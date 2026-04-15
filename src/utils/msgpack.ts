import { decode } from '@msgpack/msgpack';
import type { EventBusEvent } from '../api/events';

export type { EventBusEvent };

export function decodePayload<T>(event: EventBusEvent): T {
  console.log('[decodePayload] Decoding event:', event.topic, 'encoding:', event.encoding);
  
  if (event.encoding === 'json') {
    try {
      const parsed = JSON.parse(event.payload) as T;
      console.log('[decodePayload] JSON parsed successfully:', parsed);
      return parsed;
    } catch (err) {
      console.error('[decodePayload] JSON parse error:', err, 'payload:', event.payload);
      throw err;
    }
  }

  if (event.encoding === 'msgpack+base64') {
    try {
      console.log('[decodePayload] Base64 payload length:', event.payload.length);
      const binaryString = atob(event.payload);
      console.log('[decodePayload] Binary string length:', binaryString.length);
      const bytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }
      console.log('[decodePayload] Uint8Array first 20 bytes:', Array.from(bytes.slice(0, 20)));
      const decoded = decode(bytes) as T;
      console.log('[decodePayload] MsgPack decoded successfully:', decoded);
      return decoded;
    } catch (err) {
      console.error('[decodePayload] MsgPack decode error:', err);
      throw err;
    }
  }

  throw new Error(`Unknown encoding: ${event.encoding}`);
}
