import { render } from "@testing-library/svelte";
import { expect, test } from "vitest";
import { ICONS, type IconName } from "../../icons";
import Icon from "./Icon.svelte";

test("draws an icon at its own size and stroke width", () => {
  const { container } = render(Icon, { name: "check" });
  const svg = container.querySelector("svg")!;
  expect(svg).toHaveAttribute("width", "16");
  expect(svg).toHaveAttribute("height", "16");
  expect(svg).toHaveAttribute("stroke-width", "2.4");
  expect(svg).toHaveAttribute("viewBox", "0 0 24 24");
  expect([...svg.children].map((c) => c.getAttribute("d"))).toEqual(["M5 12.5l4.5 4.5L19 7.5"]);
});

test("size and stroke can be overridden", () => {
  const { container } = render(Icon, { name: "check", size: 10, stroke: 3 });
  const svg = container.querySelector("svg")!;
  expect(svg).toHaveAttribute("width", "10");
  expect(svg).toHaveAttribute("stroke-width", "3");
});

test("every icon draws all of its shapes with their attributes", () => {
  for (const name of Object.keys(ICONS) as IconName[]) {
    const { container, unmount } = render(Icon, { name });
    const drawn = [...container.querySelector("svg")!.children];
    expect(drawn.map((el) => el.tagName), name).toEqual(ICONS[name].shapes.map(([tag]) => tag));
    ICONS[name].shapes.forEach(([, attrs], i) => {
      for (const [k, v] of Object.entries(attrs)) expect(drawn[i].getAttribute(k), `${name} ${k}`).toBe(v);
    });
    unmount();
  }
});
