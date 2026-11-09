// Per-run state: clearing it between offloads, and the speed and time-left estimate.

import { S } from "./state.svelte.ts";

let samples: [number, number][] = [];

export function resetRun() {
  S.ejected = false;
  S.copyP = S.bakP = 0;
  S.cur = S.curB = null;
  S.rate = 0;
  S.eta = null;
  S.released = false;
  S.failCopy = S.failBak = 0;
  S.abortReason = null;
  S.stoppedKept = 0;
  S.report = null;
  S.startedAt = S.finishedAt = null;
  samples = [];
}

export function resetSamples() {
  samples = [];
}

export function sample(done: number, total: number) {
  const now = performance.now();
  samples.push([now, done]);
  while (samples.length > 2 && now - samples[0][0] > 4000) samples.shift();
  const [t0, d0] = samples[0];
  const dt = (now - t0) / 1000;
  if (dt > 0.6 && done > d0) {
    const work = (done - d0) / dt; // copy + verify bytes per second
    S.rate = work / 2; // the effective offload speed
    S.eta = (total - done) / work;
  }
}
