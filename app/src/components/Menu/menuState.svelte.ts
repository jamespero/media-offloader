// The one open menu. Any menu closes the last, and they all share a position and a frame.

import type { Component } from "svelte";

export interface OpenMenu {
  component: Component<any>;
  props: Record<string, unknown>;
  anchor: DOMRect;
  align: "left" | "right";
}

let current = $state.raw<OpenMenu | null>(null);

export const openedMenu = () => current;

export function closeMenus() {
  current = null;
}

export function openMenu(component: Component<any>, props: Record<string, unknown>, anchor: Element, align: "left" | "right" = "left") {
  current = { component, props, anchor: anchor.getBoundingClientRect(), align };
}
