import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

/**
 * Loads a component's CSS from the file named in `<style src="./Component.css"></style>`, so
 * each component keeps its styles in its own file and they're still scoped to it.
 */
const styleSrc = {
  name: "style-src",
  async style({ attributes, filename }) {
    if (typeof attributes.src !== "string" || !filename) return;
    const path = resolve(dirname(filename), attributes.src);
    const { src: _src, ...rest } = attributes;
    return { code: await readFile(path, "utf8"), dependencies: [path], attributes: rest };
  },
};

export default { preprocess: [styleSrc] };
