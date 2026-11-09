// Opens each of the app's menus from the control that was clicked.

import AddTokenMenu from "./components/Menus/AddTokenMenu.svelte";
import ChipMenu from "./components/Menus/ChipMenu.svelte";
import DestMenu from "./components/Menus/DestMenu.svelte";
import MainMenu from "./components/Menus/MainMenu.svelte";
import PresetMenu from "./components/Menus/PresetMenu.svelte";
import SourceMenu from "./components/Menus/SourceMenu.svelte";
import { openMenu } from "./components/Menu/menuState.svelte.ts";

export const mainMenu = (a: Element) => openMenu(MainMenu, {}, a, "right");
export const srcMenu = (a: Element) => openMenu(SourceMenu, {}, a);
export const destMenu = (a: Element, which: "copy" | "bak") => openMenu(DestMenu, { which }, a);
export const addTokenMenu = (a: Element) => openMenu(AddTokenMenu, {}, a);
export const chipMenu = (a: Element, index: number, onEdit: () => void) => openMenu(ChipMenu, { index, onEdit }, a);
export const presetMenu = (a: Element) => openMenu(PresetMenu, {}, a, "right");
