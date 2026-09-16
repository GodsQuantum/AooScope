export const CANVAS_WIDTH = 960;
export const CANVAS_HEIGHT = 376;
export type WidgetType = 'text' | 'value' | 'gauge' | 'ring' | 'bar' | 'badge' | 'sparkline' | 'image' | 'animation' | 'shooting_star';
export type Metric = { id: string; label: string; provider_name: string; category: string; value?: unknown; demo_value?: unknown; unit: string; recommended_widgets: string[]; online?: boolean };
export type Rect = { x: number; y: number; width: number; height: number };
export type Layer = Rect & { id: string; type: WidgetType; binding?: string; z?: number; text?: string; color?: string; [key: string]: unknown };
export type Page = { id: string; name: string; revision: number; background?: { color: string }; layers: Layer[]; [key: string]: unknown };

export const clamp = (n: number, min: number, max: number) => Math.max(min, Math.min(max, n));
export function clampRect(rect: Rect): Rect {
  const width = clamp(Math.round(rect.width), 1, CANVAS_WIDTH);
  const height = clamp(Math.round(rect.height), 1, CANVAS_HEIGHT);
  return {
    x: clamp(Math.round(rect.x), 0, CANVAS_WIDTH - width),
    y: clamp(Math.round(rect.y), 0, CANVAS_HEIGHT - height),
    width,
    height
  };
}
export function resizeRect(rect: Rect, delta: { x: number; y: number }, aspectRatio?: number): Rect {
  if (!aspectRatio || !Number.isFinite(aspectRatio) || aspectRatio <= 0) return clampRect({ ...rect, width: rect.width + delta.x, height: rect.height + delta.y });
  const widthDriven = Math.abs(delta.x / Math.max(rect.width, 1)) >= Math.abs(delta.y / Math.max(rect.height, 1));
  let width = widthDriven ? Math.max(1, Math.round(rect.width + delta.x)) : Math.max(1, Math.round((rect.height + delta.y) * aspectRatio));
  let height = Math.max(1, Math.round(width / aspectRatio));
  const maxWidth = CANVAS_WIDTH - rect.x;
  const maxHeight = CANVAS_HEIGHT - rect.y;
  if (width > maxWidth || height > maxHeight) {
    const scale = Math.min(maxWidth / width, maxHeight / height);
    width = Math.max(1, Math.floor(width * scale));
    height = Math.max(1, Math.round(width / aspectRatio));
  }
  return clampRect({ ...rect, width, height });
}
export function inversePointer(canvas: { left: number; top: number; width: number; height: number }, clientX: number, clientY: number) {
  return {
    x: Math.round(clamp((clientX - canvas.left) * CANVAS_WIDTH / canvas.width, 0, CANVAS_WIDTH)),
    y: Math.round(clamp((clientY - canvas.top) * CANVAS_HEIGHT / canvas.height, 0, CANVAS_HEIGHT))
  };
}
export function compatibleMetrics(metrics: Metric[], widget: WidgetType) { return metrics.filter((metric) => metric.recommended_widgets.includes(widget)); }

let widgetIdCounter = 0;
export function createWidgetId(randomUUID: (() => string) | null = globalThis.crypto?.randomUUID?.bind(globalThis.crypto), now = Date.now) {
  return `widget-${randomUUID ? randomUUID() : `${now()}-${++widgetIdCounter}`}`;
}
export function isLatestRequest(request: number, latest: number) { return request === latest; }
