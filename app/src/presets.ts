// Presets: a saved subfolder pattern, one per camera or recorder. Destinations
// aren't part of a preset: they're the project's
// folders, chosen once per project, and a preset's pattern goes under them. The active
// preset is whichever one matches the current pattern; any change makes it "Custom" until
// saved again.

import { ic } from "./icons";
import { closeOverlay, mi, openMenu, overlay } from "./menus";
import { esc } from "./render";
import { S, running, saveSettings, sep, tokenContext } from "./state";
import { tokenText, type Token } from "./tokens";

export interface Preset {
  id: string;
  name: string;
  tokens: Token[];
}

const sameTokens = (a: Token[], b: Token[]) => JSON.stringify(a) === JSON.stringify(b);

export function activePreset(): Preset | undefined {
  const s = S.settings;
  return s.presets.find((p) => sameTokens(p.tokens, s.tokens));
}

export function presetButton() {
  return `${ic.star(13)}<span>Preset</span><b>${esc(activePreset()?.name ?? "Custom")}</b>${ic.chev()}`;
}

export function presetMenu(anchor: Element) {
  const active = activePreset();
  const list = S.settings.presets.length
    ? S.settings.presets
        .map((p) =>
          mi("star", esc(p.name), "usepreset", active?.id === p.id ? `<span class="ck">${ic.check(14)}</span>` : "", `data-id="${esc(p.id)}" ${running() ? "disabled" : ""}`),
        )
        .join("")
    : `<div class="mnote">No presets yet. Set up the subfolders for a camera, then save them here.</div>`;
  openMenu(
    anchor,
    `<div class="mh">Presets</div>${list}<div class="sep"></div>
     ${mi("plus", "Save Current as Preset…", "savepreset", "", S.settings.tokens.length ? "" : "disabled")}
     ${mi("gear", "Manage Presets…", "managepresets")}`,
    "right",
  );
}

export function applyPreset(id: string) {
  const p = S.settings.presets.find((x) => x.id === id);
  if (!p || running()) return;
  S.settings.tokens = structuredClone(p.tokens);
  saveSettings();
}

export function savePresetDialog() {
  const suggestion = activePreset()?.name ?? (S.src ? `${S.src.label}` : "");
  overlay(`<div class="dialog"><h3>Save preset</h3>
    <p>Saves the subfolder pattern, so the next card from this camera is one click. The destinations aren't saved: they stay as they are, set once per project.</p>
    <div class="field"><input class="tin" id="presetName" placeholder="e.g. Nikon ZR" value="${esc(suggestion)}" spellcheck="false"></div>
    <div class="acts"><button class="btn" data-act="closeov">Cancel</button><button class="btn primary compact" data-act="dosavepreset">Save</button></div></div>`);
  const input = document.getElementById("presetName") as HTMLInputElement;
  input.focus();
  input.select();
  input.onkeydown = (e) => {
    if (e.key === "Enter") (document.querySelector('[data-act="dosavepreset"]') as HTMLElement).click();
  };
}

/** Saves the current setup under `name`; an existing preset with that name is updated. */
export function saveCurrentAs(name: string) {
  name = name.trim();
  if (!name) return false;
  const s = S.settings;
  const preset: Preset = { id: crypto.randomUUID(), name, tokens: structuredClone(s.tokens) };
  const i = s.presets.findIndex((p) => p.name.toLowerCase() === name.toLowerCase());
  if (i >= 0) s.presets[i] = { ...preset, id: s.presets[i].id };
  else s.presets.push(preset);
  saveSettings();
  closeOverlay();
  return true;
}

export function deletePreset(id: string) {
  S.settings.presets = S.settings.presets.filter((p) => p.id !== id);
  saveSettings();
}

/** The Naming & Presets settings tab. */
export function namingPane(set: (b: string, sub: string, ctl: string) => string) {
  const ctx = tokenContext();
  const rows = S.settings.presets.map((p) => {
    const folders = p.tokens.map((t) => esc(tokenText(t, ctx))).join(esc(sep));
    return set(
      esc(p.name),
      `<span class="mono">${folders ? `…${esc(sep)}${folders}` : "no subfolders"}</span>`,
      `<button class="btn sm" data-act="delpreset" data-id="${esc(p.id)}" title="Delete preset">${ic.trash(13)}</button>`,
    );
  });
  const n = String(S.settings.importNo).padStart(3, "0");
  return `<h5>naming &amp; presets</h5><p class="intro">A preset saves a subfolder pattern, e.g. one per camera. It goes under the copy and backup destinations, which you set once per project. Pick a preset from the ★ Preset button above the thumbnails.</p>
    ${rows.join("") || `<div class="set"><div class="l"><span>No presets yet. Use ★ Preset → Save Current as Preset….</span></div></div>`}
    ${set(
      "Import # counter",
      "The next import number. Counts up after each completed offload.",
      `<div class="row tight"><button class="btn sm" data-act="importdec" aria-label="Lower">−</button><span class="meta mono stepper-value">${n}</span><button class="btn sm" data-act="importinc" aria-label="Raise">+</button></div>`,
    )}`;
}
