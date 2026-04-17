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
      const decoded = decode(bytes);
      console.log('[decodePayload] MsgPack decoded (raw):', decoded);
      
      if (Array.isArray(decoded)) {
        const converted = convertArrayToObject(decoded, event.topic);
        console.log('[decodePayload] Converted array to object:', converted);
        return converted as T;
      }
      
      return decoded as T;
    } catch (err) {
      console.error('[decodePayload] MsgPack decode error:', err);
      throw err;
    }
  }

  throw new Error(`Unknown encoding: ${event.encoding}`);
}

function convertArrayToObject(arr: unknown[], topic: string): Record<string, unknown> {
  switch (topic) {
    case 'serial:data':
      return {
        device_id: arr[0] as string,
        data: arr[1] as number[],
        timestamp: arr[2] as number,
      };
    case 'ble:data':
      return {
        device_id: arr[0] as string,
        address: arr[1] as string,
        characteristic_uuid: arr[2] as string,
        data: arr[3] as number[],
        timestamp: arr[4] as number,
      };
    case 'gh3036:frame':
      return {
        function_id: arr[0] as number,
        function_name: arr[1] as string,
        frame_id: arr[2] as number,
        timestamp: arr[3] as number,
        channel_count: arr[4] as number,
        channels: arr[5] as number[],
      };
    case 'gh3036:frames':
      return {
        function_id: arr[0] as number,
        function_name: arr[1] as string,
        frame_count: arr[2] as number,
        channel_count: arr[3] as number,
        timestamps: arr[4] as number[],
        frame_ids: arr[5] as number[],
        channels: arr[6] as number[][],
        rawdata: arr[7] as number[][],
        gs_data: arr[8] as number[][],
      };
    case 'protocol:parsed':
      return {
        plugin_id: arr[0] as string,
        device_id: arr[1] as string,
        original_data: arr[2] as number[],
        parsed_data: arr[3],
        timestamp: arr[4] as number,
      };
    default:
      console.warn('[decodePayload] Unknown topic for array conversion:', topic);
      return { data: arr };
  }
}
