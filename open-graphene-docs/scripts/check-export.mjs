import assert from "node:assert/strict";
import { readdir, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const docsRoot = fileURLToPath(new URL("../", import.meta.url));
const contentRoot = path.join(docsRoot, "content/docs");
const outputRoot = path.join(docsRoot, "out");
const contentFiles = await readdir(contentRoot, {
  recursive: true,
});
let pageCount = 0;

for (const file of contentFiles) {
  if (!/\.mdx?$/.test(file)) {
    continue;
  }

  const route = file.replace(/\.mdx?$/, "").replace(/(^|\/)index$/, "");
  const htmlPath = path.join(outputRoot, "docs", route, "index.html");
  const markdownPath = path.join(
    outputRoot,
    "llms.mdx/docs",
    route,
    "content.md",
  );
  assert.ok((await stat(htmlPath)).size > 0, `Missing HTML: ${route}`);
  assert.ok((await stat(markdownPath)).size > 0, `Missing Markdown: ${route}`);
  pageCount += 1;
}

const outputFiles = await readdir(outputRoot, {
  recursive: true,
  withFileTypes: true,
});
let totalBytes = 0;
for (const file of outputFiles) {
  if (file.isFile()) {
    const filePath = path.join(file.parentPath, file.name);
    totalBytes += (await stat(filePath)).size;
  }
}

// GitHub Pages limits the published site to 1 GB. Fail before uploading it.
assert.ok(
  totalBytes < 1_000_000_000,
  `Static export exceeds 1 GB: ${totalBytes} bytes`,
);
assert.ok((await stat(path.join(outputRoot, ".nojekyll"))).isFile());
console.log(
  `Verified ${pageCount} HTML/Markdown page pairs; static export is ${(totalBytes / 1_000_000).toFixed(1)} MB.`,
);
