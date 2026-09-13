import type { components } from './schema';

type StatusDto = components['schemas']['StatusDto'];

export function connectStatusEvents(onStatus: (status: StatusDto) => void): () => void {
  const source = new EventSource('/api/events');
  source.addEventListener('status', (event) => {
    const message = event as MessageEvent<string>;
    onStatus(JSON.parse(message.data) as StatusDto);
  });
  return () => source.close();
}

export function connectMediaEvents(onMedia: (media: Record<string, unknown>) => void): () => void {
  const source = new EventSource('/api/events');
  source.addEventListener('media', (event) => onMedia(JSON.parse((event as MessageEvent<string>).data)));
  return () => source.close();
}
