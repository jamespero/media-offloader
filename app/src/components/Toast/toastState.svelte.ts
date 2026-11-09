// The toast on screen. A new one replaces the last.

export interface ToastMessage {
  ok: boolean;
  title: string;
  body: string;
}

const SHOWN_MS = 5000;

let current = $state.raw<ToastMessage | null>(null);

export const shownToast = () => current;

export function toast(ok: boolean, title: string, body: string) {
  const message = { ok, title, body };
  current = message;
  setTimeout(() => {
    if (current === message) current = null;
  }, SHOWN_MS);
}
