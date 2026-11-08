# Design system

How Media Offloader looks, and which values to use. The UI has to be readable at a glance from across a set, and it shows one screen of mostly status, so the rules are few and strict.

The tokens live in `app/src/styles/tokens.css`, and that file and this page should stay in step. Don't confuse design tokens with `app/src/tokens.ts`, which is the folder-naming feature.

## Rules

- **Color means status; controls stay neutral.** Violet is working, mint is done and safe, red is a problem. Buttons, chips, toggles and menus are neutral grays, and the one primary action in view is white. Never use a status color for decoration.
- **Dark only.** There's no light theme.
- **Three typefaces, all bundled:** Outfit for headings (lowercase), Inter for UI text, JetBrains Mono for paths and file names.
- **All artwork is drawn for this project.**
- **Motion explains a change of state.** Everything must still work under `prefers-reduced-motion`, which turns animation and transitions off.

## How tokens are organized

There are two layers:

1. **Base tokens** are the raw palette, such as `--gray-800`. Only `tokens.css` uses them.
2. **Semantic tokens** say what a value is for, such as `--color-surface-1`. Components use these.

To restyle something everywhere it appears, change what a semantic token points at. To change a color across the whole app, change the base token.

### Naming

- Lowercase and hyphenated, starting with the category: `--color-`, `--space-`, `--radius-`, `--font-size-`, `--shadow-`, `--z-`, `--duration-`.
- Semantic names describe the role, never the value or the appearance: `--color-status-problem`, not `--color-red`.
- Spacing steps are multiples of 2px: `--space-4` is 8px.
- A custom property that belongs to one component, such as a progress value, lives in that component and has a short name of its own: `--p`.

### Using them

- All CSS lives in CSS files. Use a class, Svelte's `class:` directive or a `data-*` attribute for states.
- If code has to supply a computed value, such as a progress width, set only a custom property (`style:--p={pct}`), and keep the rule that uses it in the CSS file.
- Literal values are fine only for the exceptions listed under [Values that stay local](#values-that-stay-local).

## Tokens

### Color

```css
:root {
  /* ---- base ---- */
  --gray-925: #0E0D12;
  --gray-900: #111015;
  --gray-875: #15141B;
  --gray-850: #17161C;
  --gray-825: #1A1920;
  --gray-800: #201E26;
  --gray-750: #2A2832;
  --gray-700: #35323E;
  --gray-650: #413E4B;
  --gray-600: #4E4B5C;
  --gray-500: #75728A;
  --gray-450: #8C899A;
  --gray-300: #ABA8B9;
  --gray-50: #F3F2F8;
  --white: #FFFFFF;
  --violet-400: #8E7CFF;
  --violet-500: #6F5CF0;
  --mint-400: #4BE3A3;
  --mint-950: #072619;
  --red-400: #FF5C5C;
  --red-500: #E8484D;

  /* ---- surfaces, darkest to lightest ---- */
  --color-surface-well: var(--gray-925);       /* inset areas: logs, re-verify list */
  --color-surface-app: var(--gray-900);        /* window background */
  --color-surface-0: var(--gray-850);          /* source panel */
  --color-surface-thumb: var(--gray-825);      /* thumbnail tiles */
  --color-surface-1: var(--gray-800);          /* panels, dialogs */
  --color-surface-2: var(--gray-750);          /* controls */
  --color-surface-3: var(--gray-700);          /* hover and selected states */
  --color-surface-4: var(--gray-650);          /* hover on buttons, toggle tracks */
  --color-surface-menu: rgba(30, 28, 36, .97);
  --color-surface-tooltip: rgba(13, 12, 17, .96);
  --color-scrim: rgba(6, 5, 9, .55);           /* behind dialogs */
  --color-thumb-shade: rgba(17, 16, 21, .62);  /* dims a thumbnail */
  --color-thumb-caption: rgba(0, 0, 0, .7);    /* behind a thumbnail's file name */
  --color-thumb-badge: rgba(0, 0, 0, .5);      /* behind a thumbnail's file type */

  /* ---- text ---- */
  --color-text: var(--gray-50);
  --color-text-secondary: var(--gray-300);
  --color-text-tertiary: var(--gray-500);
  --color-text-placeholder: var(--gray-600);   /* empty-state heading */
  --color-text-on-light: var(--gray-875);      /* on the white primary button */
  --color-text-on-status: var(--white);        /* on violet and red panels */
  --color-text-on-status-2: rgba(255, 255, 255, .85);
  --color-text-on-status-3: rgba(255, 255, 255, .62);
  --color-text-on-done: var(--mint-950);       /* on the mint panel */
  --color-text-on-done-2: rgba(7, 38, 25, .72);

  /* ---- borders ---- */
  --color-border: rgba(255, 255, 255, .07);
  --color-border-strong: rgba(255, 255, 255, .12);
  --color-border-on-status: rgba(255, 255, 255, .25);
  --color-border-working: rgba(142, 124, 255, .55);
  --color-focus-ring: var(--white);

  /* ---- status: the only colors that carry meaning ---- */
  --color-status-working: var(--violet-400);
  --color-status-working-deep: var(--violet-500);    /* start of the progress fill */
  --color-status-working-soft: rgba(142, 124, 255, .16);
  --color-status-done: var(--mint-400);
  --color-status-done-soft: rgba(75, 227, 163, .14);
  --color-status-problem: var(--red-400);
  --color-status-problem-deep: var(--red-500);       /* destructive button */
  --color-status-problem-soft: rgba(255, 92, 92, .15);

  /* ---- controls and overlays on status panels ---- */
  --color-primary: var(--gray-50);             /* the white primary button */
  --color-primary-hover: var(--white);
  --color-toggle-knob: var(--gray-450);
  --color-overlay-on-status: rgba(0, 0, 0, .18);
  --color-overlay-on-status-hover: rgba(0, 0, 0, .3);
  --color-overlay-on-done: rgba(7, 38, 25, .12);
  --color-overlay-on-done-hover: rgba(7, 38, 25, .2);
}
```

### Type

```css
:root {
  --font-heading: 'Outfit', system-ui, sans-serif;
  --font-ui: 'Inter', system-ui, sans-serif;
  --font-mono: 'JetBrains Mono', ui-monospace, monospace;

  --font-weight-regular: 400;
  --font-weight-medium: 500;
  --font-weight-semibold: 600;

  --line-height-tight: 1;     /* large numbers and headings */
  --line-height-ui: 1.4;      /* body default */
  --line-height-code: 1.6;    /* logs and lists of paths */

  --tracking-tighter: -.03em; /* the big percentage */
  --tracking-tight: -.015em;  /* headings */
  --tracking-wide: .04em;
  --tracking-caps: .07em;     /* small uppercase labels */

  /* UI text */
  --font-size-2xs: 10px;
  --font-size-xs: 11px;
  --font-size-sm: 12px;
  --font-size-md: 13px;       /* body default */
  --font-size-lg: 15px;

  /* headings and display */
  --font-size-xl: 18px;
  --font-size-2xl: 20px;
  --font-size-3xl: 24px;      /* source panel heading */
  --font-size-4xl: 30px;      /* copy and backup panel headings */
  --font-size-5xl: 34px;      /* empty state heading */
  --font-size-6xl: 44px;      /* result percentage */
  --font-size-7xl: 56px;      /* running percentage */
}
```

### Spacing, shape and size

```css
:root {
  /* spacing: --space-n is n × 2px */
  --space-1: 2px;
  --space-2: 4px;
  --space-3: 6px;
  --space-4: 8px;
  --space-5: 10px;
  --space-6: 12px;
  --space-7: 14px;
  --space-8: 16px;
  --space-9: 18px;
  --space-10: 20px;
  --space-12: 24px;
  --space-30: 60px;           /* empty states */

  /* radius */
  --radius-xs: 4px;
  --radius-sm: 6px;           /* controls inside a group */
  --radius-md: 8px;           /* controls; a group's radius is its children's + its padding */
  --radius-lg: 10px;
  --radius-xl: 12px;          /* menus, toasts */
  --radius-panel: 14px;
  --radius-dialog: 16px;
  --radius-full: 999px;       /* pills and toggles */
  --radius-circle: 50%;

  /* control heights */
  --control-height-sm: 26px;
  --control-height-md: 30px;
  --control-height-lg: 34px;  /* the primary button */
}
```

### Elevation and layers

```css
:root {
  --shadow-dialog: 0 30px 80px rgba(0, 0, 0, .6);
  --shadow-menu: 0 22px 60px rgba(0, 0, 0, .55);
  --shadow-toast: 0 18px 50px rgba(0, 0, 0, .5);
  --shadow-tooltip: 0 10px 30px rgba(0, 0, 0, .5);

  /* layers across the whole window, lowest first */
  --z-toast: 30;
  --z-scrim: 40;
  --z-menu: 60;
  --z-tooltip: 80;
}
```

### Motion and state

```css
:root {
  --duration-fast: 150ms;      /* menus, scrims */
  --duration-base: 200ms;      /* toggles, progress */
  --duration-moderate: 300ms;  /* toasts, thumbnail borders */
  --duration-slow: 450ms;      /* a panel changing state */

  --opacity-disabled: .45;
  --opacity-inactive: .38;     /* a panel that isn't available yet */
}
```

## Values that stay local

These stay as literals in their component's CSS, because a token would add a name without adding meaning:

- **Stacking inside a component:** `z-index` from 1 to 7 within a panel or thumbnail. Only window-wide layers are tokens.
- **Keyframes and looping animations:** the pulse ring and progress sheen colors, flash and glow shadows, and the durations of `spin`, `pulse`, `sheen` and `flash`.
- **One-off dimensions:** layout sizes such as the 260px top band, 236px source column and dialog widths, plus icon sizes, 1px hairlines and small nudges of 1–2px.
- **One-off opacities and filters:** for example, dimming a thumbnail.
- **The thumbnail placeholder gradient.**

## Merges

Most of the values were already consistent. These near-duplicates are the ones the scales above merge.

### Applied with the tokens

These are small enough that nobody should notice, and the tokens above already reflect them. If any turns out to be visible, keep the old value and record why here.

| From | To | Where |
|---|---|---|
| `#2a2833` | `--color-surface-2` (`#2A2832`) | Thumbnail placeholder gradient |
| `#0B0A0E` | `--color-surface-well` (`#0E0D12`) | Re-verify list |
| `rgba(255,255,255,.78)` | `--color-text-on-status-2` (`.85`) | Secondary text on the running panel |
| 11px radius | `--radius-full` | Toggle track; 21px tall, so it's already a full pill |
| 120ms, 250ms, 400ms | 150ms, 300ms, 450ms | Menu pop, toast rise, thumbnail fade |
| `-.01em` | `--tracking-tight` (`-.015em`) | Empty state heading |
| `.06em`, `.08em` | `--tracking-caps` (`.07em`) | Thumbnail type, tagline |
| `1.65` | `--line-height-code` (`1.6`) | Log lines |
| Opacity `.4`, `.5` | `--opacity-disabled` (`.45`) | Disabled menu item, locked toggle |

### Accepted on purpose

These can be seen in a side-by-side comparison, by up to a pixel or half a point. They were reviewed and accepted, because a tidy scale is the point of this work, and they land separately from the rest of #25 so any visual change is easy to review.

| # | From | To | Where | Risk to look for |
|---|---|---|---|---|
| 1 | Font 11.5px (15 uses) | 12px | Chips, previews, status text, footer | Tight rows could truncate |
| 2 | Font 12.5px (4 uses) | 13px | Buttons, panel subtext, banners | Buttons get slightly wider |
| 3 | Font 9, 9.5, 10.5, 14px | 10, 10, 11, 13px | Thumbnail badges, tooltips, menu headers, re-verify result | Thumbnail badges are tight |
| 4 | Font 21, 28px | 20, 30px | Dialog title, re-verify percentage | Low |
| 5 | Radius 7 and 9px, together | 6 and 8px | Chips and segmented buttons, buttons and groups | Keep them together, so nested corners stay concentric |
| 6 | Radius 5, 11px (menu) | 4, 12px | Tab close button, menu | Low |
| 7 | Spacing 3, 5, 7, 9, 11, 13, 22px | 4, 6, 8, 8, 10, 14, 24px | Mostly chips, pills and small gaps; 13 is settings rows, 22 is dialog padding | Dialogs and settings rows get a little roomier |

## Contrast

Measured against WCAG, which asks for 4.5:1 for normal text and 3:1 for large text. None of this changes with the tokens, and fixing it would be a separate design decision. Because the status has to be readable across a set, the first two matter most.

| Text | On | Ratio | Note |
|---|---|---|---|
| White | Violet running panel | 3.24:1 | Fine for the large percentage, low for small text |
| White | Red error panel | 3.03:1 | Same; secondary text at 85% is lower still |
| `--color-text-tertiary` | Panels (`--color-surface-1`) | 3.55:1 | Low for small text, and this is the most used text color |
| `--color-text-tertiary` | Controls (`--color-surface-2`) | 3.13:1 | Low |
| `--color-text-placeholder` | Window background | 2.24:1 | The "Waiting for media" heading, low even at its size |

These pass comfortably: `--color-text` and `--color-text-secondary` on every surface, and `--color-text-on-done` on the mint panel at 9.86:1.
