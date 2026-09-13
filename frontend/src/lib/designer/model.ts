export const CANVAS_WIDTH = 960;
export const CANVAS_HEIGHT = 376;
export type WidgetType = 'text' | 'value' | 'gauge' | 'ring' | 'bar' | 'badge' | 'sparkline' | 'image' | 'animation';
export type Metric = { id: string; label: string; provider_name: string; category: string; value?: unknown; demo_value?: unknown; unit: string; recommended_widgets: string[]; online?: boolean };
export type Rect = { x: number; y: number; width: number; height: number };
export type Layer = Rect & { id: string; type: WidgetType; binding?: string; z?: number; text?: string; color?: string; [key: string]: unknown };
export type Page = { id: string; name: string; revision: number; background?: { color: string }; layers: Layer[]; [key: string]: unknown };

export const clamp = (n: number, min: number, max: number) => Math.max(min, Math.min(max, n));
export function clampRect(rect: Rect): Rect {
  const width = clamp(rect.width, 1, CANVAS_WIDTH);
  const height = clamp(rect.height, 1, CANVAS_HEIGHT);
  return { x: clamp(rect.x, 0, CANVAS_WIDTH - width), y: clamp(rect.y, 0, CANVAS_HEIGHT - height), width, height };
}
export function resizeRect(rect: Rect, delta: { x: number; y: number }): Rect {
  return clampRect({ ...rect, width: rect.width + delta.x, height: rect.height + delta.y });
}
export function inversePointer(canvas: { left: number; top: number; width: number; height: number }, clientX: number, clientY: number) {
  return { x: clamp((clientX - canvas.left) * CANVAS_WIDTH / canvas.width, 0, CANVAS_WIDTH), y: clamp((clientY - canvas.top) * CANVAS_HEIGHT / canvas.height, 0, CANVAS_HEIGHT) };
}
export function compatibleMetrics(metrics: Metric[], widget: WidgetType) { return metrics.filter((metric) => metric.recommended_widgets.includes(widget)); }

let widgetIdCounter = 0;
export function createWidgetId(randomUUID: (() => string) | null = globalThis.crypto?.randomUUID?.bind(globalThis.crypto), now = Date.now) {
  return `widget-${randomUUID ? randomUUID() : `${now()}-${++widgetIdCounter}`}`;
}
export function isLatestRequest(request: number, latest: number) { return request === latest; }
