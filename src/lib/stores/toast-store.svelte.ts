import { addLog } from "./debug-store.svelte";

export type ToastType = "success" | "error" | "info";

export type ToastOptions = {
  duration?: number;
  detail?: string;
  action?: {
    label: string;
    onClick: () => void | Promise<void>;
  };
};

export type ToastItem = {
  id: number;
  type: ToastType;
  message: string;
  detail?: string;
  action?: ToastOptions["action"];
  closing: boolean;
};

const MAX_VISIBLE = 3;
const DEFAULT_DURATION = 5000;
const ERROR_DURATION = 8000;

let nextId = 0;
let toasts: ToastItem[] = $state([]);
const timers = new Map<number, ReturnType<typeof setTimeout>>();

export function getToasts(): ToastItem[] {
  return toasts;
}

export function showToast(type: ToastType, message: string, durationOrOptions?: number | ToastOptions) {
  const id = nextId++;
  const options: ToastOptions = typeof durationOrOptions === "number"
    ? { duration: durationOrOptions }
    : durationOrOptions ?? {};
  const ms = options.duration ?? (type === "error" ? ERROR_DURATION : DEFAULT_DURATION);

  addLog(type === "error" ? "error" : "info", "system", message);

  toasts = [...toasts, { id, type, message, detail: options.detail, action: options.action, closing: false }];

  while (toasts.filter((t) => !t.closing).length > MAX_VISIBLE) {
    const oldest = toasts.find((t) => !t.closing);
    if (oldest) dismissToast(oldest.id);
  }

  timers.set(
    id,
    setTimeout(() => dismissToast(id), ms),
  );
}

export function dismissToast(id: number) {
  const timer = timers.get(id);
  if (timer) {
    clearTimeout(timer);
    timers.delete(id);
  }

  toasts = toasts.map((t) => (t.id === id ? { ...t, closing: true } : t));

  setTimeout(() => {
    toasts = toasts.filter((t) => t.id !== id);
  }, 200);
}
