// The one open dialog or sheet, and what to run when it closes.

import type { Component } from "svelte";

export interface OpenOverlay {
  component: Component<any>;
  props: Record<string, unknown>;
}

let current = $state.raw<OpenOverlay | null>(null);
let onClose: (() => void) | null = null;

export const openedOverlay = () => current;

/** Runs `f` once when the current overlay closes (Escape, click outside, Close). */
export function whenOverlayCloses(f: () => void) {
  onClose = f;
}

export function closeOverlay() {
  current = null;
  const f = onClose;
  onClose = null;
  f?.();
}

export function overlay(component: Component<any>, props: Record<string, unknown> = {}) {
  closeOverlay();
  current = { component, props };
}
