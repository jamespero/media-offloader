// The footer's state, and the bridge that lets code outside Svelte update it. The rest of
// the UI still renders by hand, so it calls setFooter() rather than passing props.

import { mount } from "svelte";
import type { IconName } from "../../icons";
import Footer from "./Footer.svelte";

export interface FooterProps {
  /** The source or past offload shown; with none, the footer says "No source". */
  title: string | null;
  detail: string;
  /** File count and total size; empty until they're known. */
  count: string;
  items: { icon?: IconName; text: string }[];
}

const props: FooterProps = $state({ title: null, detail: "", count: "", items: [] });

export function setFooter(next: FooterProps) {
  Object.assign(props, next);
}

export function mountFooter(target: HTMLElement) {
  mount(Footer, { target, props });
}
