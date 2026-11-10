// Editing the subfolder pattern: the chips under the copy destination.

import { tokensChanged } from "./destinations";
import { S } from "./state.svelte.ts";
import type { Token, TokenType } from "./tokens";

/** Adds a subfolder of type `t` at the end, with its usual starting value. */
export function addToken(t: TokenType) {
  const tk: Token =
    t === "now" || t === "shot"
      ? { t, fmt: "YY-MM-DD" }
      : t === "day"
        ? { t, fmt: "Day01" }
        : t === "text"
          ? { t, v: "text" }
          : t === "reel"
            ? { t, v: "A" }
            : t === "project"
              ? { t, v: "Project" }
              : { t };
  S.settings.tokens.push(tk);
  tokensChanged();
}

export function setTokenFormat(i: number, fmt: string) {
  S.settings.tokens[i].fmt = fmt;
  tokensChanged();
}

export function setReel(i: number, letter: string) {
  S.settings.tokens[i].v = letter;
  tokensChanged();
}

export function removeToken(i: number) {
  S.settings.tokens.splice(i, 1);
  tokensChanged();
}

/** Swaps subfolder `i` with its neighbour to the left (-1) or right (+1). */
export function moveToken(i: number, by: -1 | 1) {
  const t = S.settings.tokens;
  const j = i + by;
  [t[i], t[j]] = [t[j], t[i]];
  tokensChanged();
}

/** Raises or lowers the Import # counter, never below 1. */
export function stepImportNo(by: -1 | 1) {
  S.settings.importNo = Math.max(1, S.settings.importNo + by);
  tokensChanged();
}
