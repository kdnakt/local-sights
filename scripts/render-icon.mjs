// Renders the app icon's SVG to a 1024x1024 PNG with a transparent background
// (gate fix 15f). macOS's `qlmanage` fills the transparent corners of the
// rounded tile with white, so the PNG is drawn with resvg instead, and the
// pixels that must stay transparent are checked before the file is written:
// the four corners, and (gate fix 16b) the 100px margin around the tile, which
// the macOS app icon grid leaves empty so the icon is as big as the other
// apps' in the Dock (the tile is 824x824, centred). The centre must be opaque.
// Run with `npm run icon:render`, then regenerate the Tauri icon set with
// `npm run tauri icon src-tauri/icons/source/icon.png` (see README.md, "App icon").
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { Resvg } from "@resvg/resvg-js";

const SIZE = 1024;
/** The transparent margin on every side of the 824x824 tile (gate fix 16b). */
const MARGIN = 100;
const SOURCE_SVG = fileURLToPath(new URL("../src-tauri/icons/source/icon.svg", import.meta.url));
const OUTPUT_PNG = fileURLToPath(new URL("../src-tauri/icons/source/icon.png", import.meta.url));

/** The alpha (0 to 255) of the pixel at (x, y) in an RGBA buffer `width` pixels wide. */
function alphaAt(pixels, width, x, y) {
  return pixels[(y * width + x) * 4 + 3];
}

async function main() {
  const svg = await readFile(SOURCE_SVG, "utf8");
  // No `background` option: whatever the SVG does not paint stays transparent.
  const rendered = new Resvg(svg, { fitTo: { mode: "width", value: SIZE } }).render();
  if (rendered.width !== SIZE || rendered.height !== SIZE) {
    throw new Error(`expected ${SIZE}x${SIZE}, rendered ${rendered.width}x${rendered.height}`);
  }

  const pixels = rendered.pixels;
  const last = SIZE - 1;
  const middle = SIZE / 2;
  // Points that must be fully transparent: the corners (15f) and the middle of
  // each margin band, halfway between the canvas edge and the tile (16b).
  const transparentPoints = [
    ["top-left corner", 0, 0],
    ["top-right corner", last, 0],
    ["bottom-left corner", 0, last],
    ["bottom-right corner", last, last],
    ["left margin", MARGIN / 2, middle],
    ["right margin", last - MARGIN / 2, middle],
    ["top margin", middle, MARGIN / 2],
    ["bottom margin", middle, last - MARGIN / 2],
  ];
  const opaque = transparentPoints.filter(([, x, y]) => alphaAt(pixels, SIZE, x, y) !== 0);
  if (opaque.length > 0) {
    const names = opaque.map(([name, x, y]) => `${name} (alpha ${alphaAt(pixels, SIZE, x, y)})`);
    throw new Error(
      `the corners and the ${MARGIN}px margin must be transparent; not transparent: ${names.join(", ")}`,
    );
  }
  const centreAlpha = alphaAt(pixels, SIZE, middle, middle);
  if (centreAlpha !== 255) {
    throw new Error(`the centre must be opaque; its alpha is ${centreAlpha}`);
  }

  await writeFile(OUTPUT_PNG, rendered.asPng());
  console.log(
    `wrote ${OUTPUT_PNG} (${SIZE}x${SIZE}, transparent corners and ${MARGIN}px margin, opaque centre)`,
  );
}

main().catch((error) => {
  console.error(`render-icon: ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
});
