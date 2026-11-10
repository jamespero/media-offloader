// Line icons (24×24 grid, stroke = currentColor), same set as the mockup. Drawn by
// components/Icon: each has a default size and stroke width, and the shapes it's made of.

export type Shape = ["path" | "rect" | "circle", Record<string, string>];

export interface IconDef {
  size: number;
  stroke: number;
  shapes: Shape[];
}

export const ICONS = {
  card: { size: 36, stroke: 1.5, shapes: [["path", { d: "M7 2h8l4 4v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z" }], ["path", { d: "M9 6v3M12 6v3M15 6v3" }]] },
  cardWaiting: { size: 36, stroke: 1.5, shapes: [["path", { d: "M7 2h8l4 4v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z", "stroke-dasharray": "3 3" }]] },
  cardInsert: { size: 54, stroke: 1.2, shapes: [["path", { d: "M7 2h8l4 4v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z", "stroke-dasharray": "3 3" }], ["path", { d: "M12 9v6M9 12l3 3 3-3" }]] },
  drive: { size: 36, stroke: 1.5, shapes: [["rect", { x: "3", y: "6", width: "18", height: "12", rx: "2" }], ["path", { d: "M7 14h.01M11 14h6" }]] },
  folder: { size: 36, stroke: 1.5, shapes: [["path", { d: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" }]] },
  eject: { size: 15, stroke: 1.8, shapes: [["path", { d: "M12 5l7 9H5z" }], ["path", { d: "M5 19h14" }]] },
  check: { size: 16, stroke: 2.4, shapes: [["path", { d: "M5 12.5l4.5 4.5L19 7.5" }]] },
  x: { size: 15, stroke: 2, shapes: [["path", { d: "M6 6l12 12M18 6L6 18" }]] },
  menu: { size: 18, stroke: 1.8, shapes: [["path", { d: "M4 7h16M4 12h16M4 17h16" }]] },
  chev: { size: 12, stroke: 2.2, shapes: [["path", { d: "M6 9l6 6 6-6" }]] },
  chevL: { size: 15, stroke: 2, shapes: [["path", { d: "M15 6l-6 6 6 6" }]] },
  chevR: { size: 15, stroke: 2, shapes: [["path", { d: "M9 6l6 6-6 6" }]] },
  plus: { size: 14, stroke: 2, shapes: [["path", { d: "M12 5v14M5 12h14" }]] },
  cal: { size: 13, stroke: 1.8, shapes: [["rect", { x: "3", y: "5", width: "18", height: "16", rx: "2" }], ["path", { d: "M8 3v4M16 3v4M3 10h18" }]] },
  hash: { size: 13, stroke: 1.8, shapes: [["path", { d: "M5 9h14M5 15h14M10 4L8 20M16 4l-2 16" }]] },
  disk: { size: 13, stroke: 1.8, shapes: [["rect", { x: "3", y: "6", width: "18", height: "12", rx: "2" }], ["path", { d: "M7 14h.01" }]] },
  cam: { size: 13, stroke: 1.8, shapes: [["path", { d: "M4 8h3l2-3h6l2 3h3a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1z" }], ["circle", { cx: "12", cy: "13", r: "3.5" }]] },
  film: { size: 13, stroke: 1.8, shapes: [["rect", { x: "3", y: "4", width: "18", height: "16", rx: "2" }], ["path", { d: "M7 4v16M17 4v16M3 9h4M3 15h4M17 9h4M17 15h4" }]] },
  image: { size: 13, stroke: 1.8, shapes: [["rect", { x: "3", y: "4", width: "18", height: "16", rx: "2" }], ["circle", { cx: "9", cy: "10", r: "2" }], ["path", { d: "M21 16l-5-5-9 9" }]] },
  wave: { size: 13, stroke: 1.8, shapes: [["path", { d: "M3 12h2M7 8v8M11 5v14M15 9v6M19 7v10M21 12h0" }]] },
  tag: { size: 13, stroke: 1.8, shapes: [["path", { d: "M3 12V4h8l10 10-8 8z" }], ["circle", { cx: "7.5", cy: "8.5", r: "1" }]] },
  type: { size: 13, stroke: 1.8, shapes: [["path", { d: "M5 6V4h14v2M12 4v16M9 20h6" }]] },
  info: { size: 13, stroke: 2, shapes: [["circle", { cx: "12", cy: "12", r: "9" }], ["path", { d: "M12 11v5M12 8h.01" }]] },
  alert: { size: 14, stroke: 2, shapes: [["path", { d: "M12 9v4M12 17h.01" }], ["path", { d: "M10.3 3.9L2 18a2 2 0 0 0 1.7 3h16.6a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" }]] },
  gear: { size: 15, stroke: 1.6, shapes: [["circle", { cx: "12", cy: "12", r: "3" }], ["path", { d: "M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" }]] },
  refresh: { size: 15, stroke: 1.8, shapes: [["path", { d: "M3 12a9 9 0 0 1 15-6.7L21 8M21 3v5h-5M21 12a9 9 0 0 1-15 6.7L3 16M3 21v-5h5" }]] },
  file: { size: 15, stroke: 1.8, shapes: [["path", { d: "M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" }], ["path", { d: "M14 3v5h5M9 13h6M9 17h6" }]] },
  layers: { size: 15, stroke: 1.8, shapes: [["path", { d: "M12 3l9 5-9 5-9-5z" }], ["path", { d: "M3 13l9 5 9-5" }]] },
  newf: { size: 15, stroke: 1.8, shapes: [["path", { d: "M12 5v14M5 12h14" }]] },
  shield: { size: 15, stroke: 1.8, shapes: [["path", { d: "M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z" }], ["path", { d: "M9 12l2 2 4-4" }]] },
  trash: { size: 15, stroke: 1.8, shapes: [["path", { d: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" }]] },
  reveal: { size: 15, stroke: 1.8, shapes: [["path", { d: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" }], ["path", { d: "M12 11v5M9.5 13.5L12 11l2.5 2.5" }]] },
  star: { size: 15, stroke: 1.8, shapes: [["path", { d: "M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z" }]] },
  bell: { size: 15, stroke: 1.8, shapes: [["path", { d: "M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" }], ["path", { d: "M10 21h4" }]] },
  spinner: { size: 15, stroke: 2, shapes: [["path", { d: "M12 3a9 9 0 1 0 9 9" }]] },
} satisfies Record<string, IconDef>;

export type IconName = keyof typeof ICONS;
