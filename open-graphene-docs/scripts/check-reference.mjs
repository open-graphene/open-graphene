import assert from "node:assert/strict";
import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const docsRoot = fileURLToPath(new URL("../", import.meta.url));
const repositoryRoot = path.dirname(docsRoot);
const outputRoot = path.join(docsRoot, "content/docs/reference");
const inventoryPath = path.join(docsRoot, ".reference/inventory.json");
const inventory = JSON.parse(await readFile(inventoryPath, "utf8"));
const configPath = path.join(docsRoot, ".reference/tsconfig.json");
const configFile = ts.readConfigFile(configPath, ts.sys.readFile);
const config = ts.parseJsonConfigFileContent(
  configFile.config,
  ts.sys,
  path.dirname(configPath),
);
const program = ts.createProgram(config.fileNames, config.options);
const checker = program.getTypeChecker();

for (const entry of inventory) {
  const sourceFile = program.getSourceFile(
    path.join(repositoryRoot, entry.entryPoint),
  );
  assert.ok(sourceFile, `Missing source: ${entry.entryPoint}`);
  const module = checker.getSymbolAtLocation(sourceFile);
  assert.ok(module, `Missing module: ${entry.package}`);
  const publicExports = checker
    .getExportsOfModule(module)
    .map((symbol) => symbol.name)
    .sort();

  assert.deepEqual(
    [...new Set(entry.exports)].sort(),
    publicExports,
    `Incomplete export catalog: ${entry.package}`,
  );
}

const files = await readdir(outputRoot, {
  recursive: true,
});
const markdownFiles = files.filter((file) => file.endsWith(".md"));
const generatedPaths = new Set(
  markdownFiles.map((file) => path.join(outputRoot, file)),
);
const contentRoot = path.join(docsRoot, "content/docs");
const contentFiles = await readdir(contentRoot, {
  recursive: true,
});
const pageRoutes = new Set(
  contentFiles
    .filter((file) => /\.mdx?$/.test(file))
    .map((file) => {
      const route = file
        .replace(/\.mdx?$/, "")
        .replace(/(^|\/)index$/, "")
        .replace(/\/$/, "");

      return `/docs/${route}/`;
    }),
);
let checkedLinks = 0;

for (const file of markdownFiles) {
  const filePath = path.join(outputRoot, file);
  const content = await readFile(filePath, "utf8");
  assert.match(content, /^---\ntitle: /, `Missing Fumadocs title: ${file}`);

  for (const match of content.matchAll(/\]\((\/docs\/[^)#]*)(?:#[^)]*)?\)/g)) {
    assert.ok(
      pageRoutes.has(match[1]),
      `Broken page route: ${file} → ${match[1]}`,
    );
    checkedLinks += 1;
  }

  for (const match of content.matchAll(/\]\(([^)]+\.mdx?)(?:#[^)]*)?\)/g)) {
    const href = match[1];
    if (/^[a-z]+:\/\//.test(href)) {
      continue;
    }

    const destination = path.resolve(
      path.dirname(filePath),
      decodeURIComponent(href),
    );
    if (!generatedPaths.has(destination)) {
      assert.ok(
        (await stat(destination)).isFile(),
        `Broken reference: ${file} → ${href}`,
      );
    }
    checkedLinks += 1;
  }
}

// Exercise nested RPC members and aliases as well as top-level export completeness.
const clients = {
  bitshares: "BitSharesClient",
  swaplock: "SwaplockClient",
  rsquared: "RSquaredClient",
};
for (const [chain, client] of Object.entries(clients)) {
  const clientPage = await readFile(
    path.join(outputRoot, chain, "typescript/classes", `${client}.md`),
    "utf8",
  );
  for (const method of [
    "connect",
    "getObjects",
    "account",
    "prepareOperations",
    "close",
  ]) {
    assert.match(
      clientPage,
      new RegExp(`#{2,6} ${method}\\b`),
      `${client}.${method} is undocumented`,
    );
  }

  const hook = await readFile(
    path.join(outputRoot, chain, "react/functions/useAccount.md"),
    "utf8",
  );
  assert.match(hook, /nameOrId/);
  assert.match(hook, /LiveReadOptions/);
  assert.match(hook, /LiveQueryResult/);
}

console.log(
  `Verified all exports of ${inventory.length} entry points, ${markdownFiles.length} pages and ${checkedLinks} local reference links.`,
);

const examplesDirectory = path.join(docsRoot, ".reference/examples");
await mkdir(examplesDirectory, {
  recursive: true,
});

await writeFile(
  path.join(examplesDirectory, "package.json"),
  '{"type":"module"}\n',
);

const exampleFiles = [];
for (const guide of ["bitshares", "swaplock", "rsquared", "react"]) {
  const guidePath = path.join(docsRoot, "content/docs/guides", `${guide}.mdx`);
  const content = await readFile(guidePath, "utf8");
  const language = guide === "react" ? "tsx" : "ts";
  const code = content.match(
    new RegExp("```" + language + "\\n([\\s\\S]*?)```"),
  )?.[1];
  assert.ok(code, `Missing runnable example: ${guide}`);

  const examplePath = path.join(examplesDirectory, `${guide}.${language}`);
  await writeFile(examplePath, code);
  exampleFiles.push(examplePath);
}

const queryPackageRoot = path.join(
  repositoryRoot,
  "open-graphene-packages/typescript/node_modules/@tanstack/react-query",
);
const queryManifest = JSON.parse(
  await readFile(path.join(queryPackageRoot, "package.json"), "utf8"),
);

const examplesProgram = ts.createProgram(exampleFiles, {
  ...config.options,
  paths: {
    ...config.options.paths,
    "@tanstack/react-query": [path.join(queryPackageRoot, queryManifest.types)],
  },
});
const diagnostics = ts.getPreEmitDiagnostics(examplesProgram);
assert.equal(
  diagnostics.length,
  0,
  ts.formatDiagnosticsWithColorAndContext(diagnostics, {
    getCurrentDirectory: () => docsRoot,
    getCanonicalFileName: (file) => file,
    getNewLine: () => "\n",
  }),
);
console.log(
  "Typechecked the three chain connection examples and the React provider example against SDK source.",
);
