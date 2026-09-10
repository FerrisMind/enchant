/**
 * Playwright capture: Cool Mode docs idle + press-hold burst.
 * Saves ref-cool-idle.png, ref-cool-press.png, and optional custom-image press.
 */
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(path.join(__dirname, "package.json"));
const { chromium } = require("playwright");

const outDir = path.resolve(
  process.env.COOL_MODE_VISUAL_OUT ??
    path.join(
      __dirname,
      "../../.superpowers/sdd/visual",
    ),
);
const baseUrl = process.env.REF_BASE_URL ?? "http://localhost:5173";
const route = "/magic/docs/components/cool-mode";

fs.mkdirSync(outDir, { recursive: true });

function previewContainer(button) {
  return button.evaluateHandle((el) => {
    let cur = el.parentElement;
    while (cur && cur.parentElement) {
      const r = cur.getBoundingClientRect();
      if (r.height >= 220 && r.width >= 280) return cur;
      cur = cur.parentElement;
    }
    return el.parentElement ?? el;
  });
}

async function capturePressBurst(page, button, target, outPath, holdMs = 700) {
  const bbox = await button.boundingBox();
  if (!bbox) throw new Error(`button has no bounding box for ${outPath}`);
  const cx = bbox.x + bbox.width / 2;
  const cy = bbox.y + bbox.height / 2;
  await page.mouse.move(cx, cy);
  await page.mouse.down();
  await page.waitForTimeout(holdMs);
  await target.screenshot({ path: outPath });
  await page.mouse.up();
  await page.waitForTimeout(200);
  return bbox;
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
await page.goto(new URL(route, baseUrl).toString(), {
  waitUntil: "networkidle",
  timeout: 120_000,
});
await page.waitForTimeout(800);

const clickMe = page.getByRole("button", { name: "Click Me!" }).first();
await clickMe.waitFor({ state: "visible", timeout: 30_000 });
await clickMe.scrollIntoViewIfNeeded();

const clickHandle = await previewContainer(clickMe);
const clickTarget = clickHandle.asElement() ?? clickMe;

const idlePath = path.join(outDir, "ref-cool-idle.png");
await clickTarget.screenshot({ path: idlePath });

const pressPath = path.join(outDir, "ref-cool-press.png");
const pressedPath = path.join(outDir, "ref-cool-pressed.png");
const clickBox = await capturePressBurst(
  page,
  clickMe,
  clickTarget,
  pressPath,
  700,
);
fs.copyFileSync(pressPath, pressedPath);

// Full-viewport press (particles leave preview; documents no clip-to-button).
const pressFullPath = path.join(outDir, "ref-cool-press-full.png");
await page.mouse.move(clickBox.x + clickBox.width / 2, clickBox.y + clickBox.height / 2);
await page.mouse.down();
await page.waitForTimeout(700);
await page.screenshot({ path: pressFullPath, fullPage: false });
await page.mouse.up();
await page.waitForTimeout(300);

let customPressPath = null;
const svelteBtn = page.getByRole("button", { name: "Svelte Image" }).first();
if ((await svelteBtn.count()) > 0) {
  await svelteBtn.scrollIntoViewIfNeeded();
  await page.waitForTimeout(400);
  const svelteHandle = await previewContainer(svelteBtn);
  const svelteTarget = svelteHandle.asElement() ?? svelteBtn;
  customPressPath = path.join(outDir, "ref-cool-custom-press.png");
  await capturePressBurst(page, svelteBtn, svelteTarget, customPressPath, 700);
}

await browser.close();
console.log(
  JSON.stringify(
    {
      idlePath,
      pressPath,
      pressedPath,
      pressFullPath,
      customPressPath,
      clickBox,
      baseUrl,
    },
    null,
    2,
  ),
);
