import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { PNG } from "pngjs";
import pixelmatch from "pixelmatch";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const workspaceRoot = path.resolve(__dirname, "..", "..");
const catalogPath = path.join(workspaceRoot, "crates", "wasm-demo", "src", "catalog.rs");

const refBaseUrl = process.env.REF_BASE_URL ?? "http://127.0.0.1:3000";
const wasmBaseUrl = process.env.WASM_BASE_URL ?? "http://127.0.0.1:8080";
const outDir = path.resolve(process.env.OUT_DIR ?? path.join(__dirname, "artifacts"));
const maxDiffRatio = Number(process.env.MAX_DIFF_RATIO ?? "0.02");
const timeoutMs = Number(process.env.PAGE_TIMEOUT_MS ?? "120000");
const caseFilter = process.env.CASE_FILTER ?? "";

const viewports = [
  { id: "desktop", width: 1440, height: 1600 },
  { id: "mobile", width: 390, height: 844 },
];

const themes = ["light", "dark"];

function readSupportedDocSlugs() {
  const source = fs.readFileSync(catalogPath, "utf8");
  const regex = /Self::[A-Za-z0-9_]+\s*=>\s*"([^"]+)"/g;
  const slugs = [];
  let match;
  while ((match = regex.exec(source)) !== null) {
    slugs.push(match[1]);
  }
  return slugs;
}

function buildRouteMatrix() {
  const docs = readSupportedDocSlugs().map((slug) => `/docs/components/${slug}`);
  return ["/", "/components", ...docs];
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function slugifyRoute(route) {
  if (route === "/") {
    return "home";
  }

  return route
    .replace(/^\//, "")
    .replace(/[/?=&]+/g, "-")
    .replace(/-+/g, "-");
}

function withThemeQuery(route, theme) {
  const separator = route.includes("?") ? "&" : "?";
  return `${route}${separator}theme=${theme}`;
}

function refPageUrl(route) {
  return new URL(route, refBaseUrl).toString();
}

function wasmPageUrl(route, theme) {
  return new URL(withThemeQuery(route, theme), wasmBaseUrl).toString();
}

async function prepareReferencePage(page, theme) {
  await page.addInitScript((nextTheme) => {
    window.localStorage.setItem("theme", nextTheme);
    document.documentElement.className = nextTheme === "dark" ? "dark" : "";
    document.documentElement.classList.toggle("dark", nextTheme === "dark");
    document.documentElement.style.colorScheme = nextTheme;
  }, theme);
}

async function forceReferenceTheme(page, theme) {
  await page.evaluate((nextTheme) => {
    window.localStorage.setItem("theme", nextTheme);
    document.documentElement.className = nextTheme === "dark" ? "dark" : "";
    document.documentElement.classList.toggle("dark", nextTheme === "dark");
    document.documentElement.style.colorScheme = nextTheme;
  }, theme);
}

async function captureScreenshot(page, url, filePath, options = {}) {
  const { referenceTheme = null, fullPage = true } = options;
  await page.goto(url, { waitUntil: "networkidle", timeout: timeoutMs });
  if (referenceTheme) {
    await forceReferenceTheme(page, referenceTheme);
  }
  await page.waitForTimeout(1500);
  await page.screenshot({ path: filePath, fullPage });
}

async function referenceScrollHeight(page) {
  return await page.evaluate(() => {
    const root = document.scrollingElement ?? document.documentElement;
    return Math.ceil(root.scrollHeight);
  });
}

function compareImages(refPath, wasmPath, diffPath) {
  const refImage = PNG.sync.read(fs.readFileSync(refPath));
  const wasmImage = PNG.sync.read(fs.readFileSync(wasmPath));
  const width = Math.max(refImage.width, wasmImage.width);
  const height = Math.max(refImage.height, wasmImage.height);
  const normalizedRef = new PNG({ width, height });
  const normalizedWasm = new PNG({ width, height });
  PNG.bitblt(refImage, normalizedRef, 0, 0, refImage.width, refImage.height, 0, 0);
  PNG.bitblt(wasmImage, normalizedWasm, 0, 0, wasmImage.width, wasmImage.height, 0, 0);
  const diff = new PNG({ width, height });
  const diffPixels = pixelmatch(
    normalizedRef.data,
    normalizedWasm.data,
    diff.data,
    width,
    height,
    { threshold: 0.1 }
  );

  fs.writeFileSync(diffPath, PNG.sync.write(diff));

  return {
    diffPixels,
    totalPixels: width * height,
    ratio: diffPixels / (width * height),
  };
}

async function main() {
  ensureDir(outDir);
  const browser = await chromium.launch({ headless: true });
  const routes = buildRouteMatrix();
  const summary = [];

  try {
    for (const viewport of viewports) {
      for (const theme of themes) {
        for (const route of routes) {
          const caseId = `${viewport.id}-${theme}-${slugifyRoute(route)}`;
          if (caseFilter && !caseId.includes(caseFilter) && !route.includes(caseFilter)) {
            continue;
          }
          const caseDir = path.join(outDir, caseId);
          ensureDir(caseDir);

          const refPage = await browser.newPage({ viewport });
          await prepareReferencePage(refPage, theme);
          const wasmPage = await browser.newPage({ viewport });

          const refPath = path.join(caseDir, "reference.png");
          const wasmPath = path.join(caseDir, "wasm.png");
          const diffPath = path.join(caseDir, "diff.png");

          try {
            await captureScreenshot(refPage, refPageUrl(route), refPath, {
              referenceTheme: theme,
            });
            const targetHeight = Math.min(
              Math.max(await referenceScrollHeight(refPage), viewport.height),
              8192
            );
            await wasmPage.setViewportSize({
              width: viewport.width,
              height: targetHeight,
            });
            await captureScreenshot(wasmPage, wasmPageUrl(route, theme), wasmPath, {
              fullPage: false,
            });
            const result = compareImages(refPath, wasmPath, diffPath);
            summary.push({
              caseId,
              route,
              theme,
              viewport: viewport.id,
              diffPixels: result.diffPixels,
              totalPixels: result.totalPixels,
              ratio: Number(result.ratio.toFixed(6)),
              pass: result.ratio <= maxDiffRatio,
            });
          } finally {
            await refPage.close();
            await wasmPage.close();
          }
        }
      }
    }
  } finally {
    await browser.close();
  }

  const summaryPath = path.join(outDir, "summary.json");
  fs.writeFileSync(summaryPath, JSON.stringify(summary, null, 2));

  const failing = summary.filter((entry) => !entry.pass);
  for (const entry of summary) {
    console.log(
      `${entry.pass ? "PASS" : "FAIL"} ${entry.caseId} diff_ratio=${entry.ratio} diff_pixels=${entry.diffPixels}`
    );
  }
  console.log(`Summary written to ${summaryPath}`);

  if (failing.length > 0) {
    process.exitCode = 1;
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
