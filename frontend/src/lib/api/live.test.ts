import { afterEach, describe, expect, it, vi } from 'vitest';
import { connectStatusEvents } from './live';

class FakeEventSource {
  static last: FakeEventSource | undefined;
  listeners = new Map<string, (event: MessageEvent) => void>();
  closed = false;
  constructor(public url: string) { FakeEventSource.last = this; }
  addEventListener(name: string, fn: EventListener) {
    this.listeners.set(name, fn as (event: MessageEvent) => void);
  }
  close() { this.closed = true; }
  emit(name: string, data: string) {
    this.listeners.get(name)?.(new MessageEvent(name, { data }));
  }
}

afterEach(() => vi.unstubAllGlobals());

describe('connectStatusEvents', () => {
  it('decodes named status events and closes cleanly', () => {
    vi.stubGlobal('EventSource', FakeEventSource);
    const seen: unknown[] = [];
    const close = connectStatusEvents((status) => seen.push(status));
    expect(FakeEventSource.last?.url).toBe('/api/events');
    FakeEventSource.last?.emit('status', '{"brightness":61,"version":"test","native_brightness":false,"device_present":true,"updated_unix":null}');
    expect(seen).toHaveLength(1);
    close();
    expect(FakeEventSource.last?.closed).toBe(true);
  });
});
