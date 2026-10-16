import { mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Application } from "typedoc";

const docsRoot = fileURLToPath(new URL("../", import.meta.url));
const repositoryRoot = path.dirname(docsRoot);
const sdkRoot = path.join(repositoryRoot, "open-graphene-packages/typescript");
const outputRoot = path.join(docsRoot, "content/docs/reference");
const temporaryRoot = path.join(docsRoot, ".reference");
const chains = ["bitshares", "swaplock", "rsquared"];
const entries = [];
const paths = {};

async function addPackage(directory, section) {
  const packageRoot = path.join(sdkRoot, directory);
  const manifest = JSON.parse(
    await readFile(path.join(packageRoot, "package.json"), "utf8"),
  );

  for (const [subpath, target] of Object.entries(manifest.exports)) {
    const sourcePath =
      target["workspace-source"] ??
      target.types.replace("./dist/", "./src/").replace(/\.d\.ts$/, ".ts");
    const entryPoint = path.join(packageRoot, sourcePath);
    const suffix = subpath === "." ? "" : subpath.slice(1);
    const importName = `${manifest.name}${suffix}`;

    paths[importName] = [entryPoint];
    entries.push({
      entryPoint,
      importName,
      section: `${section}${suffix}`,
    });
  }
}

for (const chain of chains) {
  for (const kind of ["api", "react", "bindings"]) {
    const section = kind === "api" ? "typescript" : kind;
    await addPackage(
      `graphene-chain-${chain}/graphene-chain-${chain}-${kind}`,
      `${chain}/${section}`,
    );
  }
}

for (const name of [
  "graphene",
  "core",
  "fc",
  "transport",
  "codec",
  "primitives",
  "react-core",
]) {
  const directory = name === "graphene" ? name : `graphene-${name}`;
  await addPackage(directory, `shared/${name}`);
}

await mkdir(temporaryRoot, {
  recursive: true,
});

const tsconfigPath = path.join(temporaryRoot, "tsconfig.json");
await writeFile(
  tsconfigPath,
  JSON.stringify(
    {
      extends: path.join(sdkRoot, "tsconfig.base.json"),
      compilerOptions: {
        paths,
        noEmit: true,
        jsx: "react-jsx",
      },
      files: entries.map((entry) => entry.entryPoint),
    },
    null,
    2,
  ),
);

const application = await Application.bootstrapWithPlugins({
  entryPoints: entries.map((entry) => entry.entryPoint),
  tsconfig: tsconfigPath,
  plugin: ["typedoc-plugin-markdown"],
  name: "TypeScript and React API",
  readme: "none",
  excludePrivate: true,
  excludeProtected: true,
  excludeInternal: true,
  entryFileName: "index",
  router: "member",
  markdown: outputRoot,
  hidePageHeader: true,
  hideBreadcrumbs: true,
  useCodeBlocks: true,
  expandParameters: true,
  parametersFormat: "table",
  sourceLinkTemplate:
    "https://github.com/open-graphene/open-graphene/blob/main/{path}#L{line}",
  basePath: repositoryRoot,
  validation: {
    notExported: false,
  },
});

const project = await application.convert();
if (!project) {
  throw new Error("TypeDoc could not convert the public SDK exports.");
}

for (const module of project.children ?? []) {
  const sourcePath = module.sources?.[0]?.fullFileName;
  const entry = entries.find((item) => item.entryPoint === sourcePath);
  if (!entry) {
    throw new Error(`No package mapping for ${module.name}: ${sourcePath}`);
  }

  module.name = entry.section;
  console.log(`${entry.importName}: ${module.children?.length ?? 0} exports`);
}

await rm(outputRoot, {
  recursive: true,
  force: true,
});
await application.generateOutputs(project);
if (application.logger.hasErrors()) {
  throw new Error("API documentation generation failed.");
}
await application.generateJson(project, path.join(temporaryRoot, "api.json"));

const packageTitles = {
  typescript: "TypeScript API",
  react: "React API",
  bindings: "Protocol bindings",
};
const chainTitles = {
  bitshares: "BitShares",
  swaplock: "Swaplock",
  rsquared: "R-Squared",
};

async function writeMetadata(directory, metadata) {
  await mkdir(directory, {
    recursive: true,
  });
  await writeFile(
    path.join(directory, "meta.json"),
    `${JSON.stringify(metadata, null, 2)}\n`,
  );
}

function resolveReferenceLinks(markdown, filePath) {
  return markdown.replace(
    /\]\(([^)]+\.mdx?)(#[^)]*)?\)/g,
    (match, href, anchor = "") => {
      if (/^[a-z]+:\/\//.test(href)) {
        return match;
      }

      const destination = path.resolve(
        path.dirname(filePath),
        decodeURIComponent(href),
      );
      const relativePath = path.relative(
        path.join(docsRoot, "content/docs"),
        destination,
      );
      const route = relativePath
        .replace(/\.mdx?$/, "")
        .replace(/(^|\/)index$/, "");

      return `](/docs/${route.replace(/\/$/, "")}/${anchor})`;
    },
  );
}

const generatedFiles = await readdir(outputRoot, {
  recursive: true,
});
const markdownFiles = generatedFiles.filter((file) => file.endsWith(".md"));

const entriesBySpecificity = [...entries].sort((left, right) => {
  return right.section.length - left.section.length;
});

for (const file of markdownFiles) {
  const filePath = path.join(outputRoot, file);
  const markdown = await readFile(filePath, "utf8");
  const heading = markdown.match(/^# (.+)\n/);
  if (!heading) {
    throw new Error(`Missing page title: ${file}`);
  }

  const entry = entriesBySpecificity.find((item) => {
    return file.startsWith(`${item.section}/`);
  });
  let title = heading[1].replace(
    /^(Class|Function|Interface|Type Alias|Variable|Enumeration): /,
    "",
  );
  let introduction = "";
  if (entry && file === `${entry.section}/index.md`) {
    const sectionName = entry.section.split("/").at(-1);
    title = packageTitles[sectionName] ?? entry.importName;
    introduction = `Import from \`${entry.importName}\`. This catalog includes every public export from this entry point.\n\n`;
  }

  const description = entry
    ? `${entry.importName} · API reference`
    : "Public TypeScript and React API";
  const frontmatter = `---\ntitle: ${JSON.stringify(title)}\ndescription: ${JSON.stringify(description)}\n---\n\n`;
  await writeFile(
    filePath,
    frontmatter +
      introduction +
      resolveReferenceLinks(
        markdown.slice(heading[0].length).trimStart(),
        filePath,
      ),
  );
}

await writeMetadata(outputRoot, {
  title: "API reference",
  pages: ["index", ...chains, "shared"],
});

for (const entry of entries) {
  const childEntries = entries.filter(
    (other) => path.posix.dirname(other.section) === entry.section,
  );
  await writeMetadata(path.join(outputRoot, entry.section), {
    title:
      packageTitles[entry.section.split("/").at(-1)] ??
      entry.importName.replace("@open-graphene/", ""),
    pages: [
      "index",
      ...childEntries.map((child) => path.posix.basename(child.section)),
    ],
  });
}

for (const chain of chains) {
  const title = chainTitles[chain];
  await writeMetadata(path.join(outputRoot, chain), {
    title,
    root: true,
    pages: ["index", "typescript", "react", "bindings"],
  });
  const content =
    `---\ntitle: ${JSON.stringify(title)}\ndescription: ${JSON.stringify(`${title} SDK for TypeScript and React.`)}\n---\n\n` +
    `- [Getting started](../../guides/${chain}.mdx)\n` +
    "- [TypeScript API](./typescript/index.md): clients, queries, transactions and public helpers.\n" +
    "- [React API](./react/index.md): providers, read hooks, query-options factories and mutations.\n" +
    "- [Protocol bindings](./bindings/index.md): RPC descriptors, operations, codecs and types.\n\n" +
    "Every catalog is generated from this repository’s public package exports. Select a symbol to see its parameters, return type and source.\n";
  const chainIndexPath = path.join(outputRoot, chain, "index.md");
  await writeFile(
    chainIndexPath,
    resolveReferenceLinks(content, chainIndexPath),
  );
}

const sharedEntries = entries.filter((entry) =>
  entry.section.startsWith("shared/"),
);
await writeMetadata(path.join(outputRoot, "shared"), {
  title: "Shared packages",
  root: true,
  pages: [
    "index",
    "graphene",
    "core",
    "react-core",
    "transport",
    "fc",
    "codec",
    "primitives",
  ],
});
await writeFile(
  path.join(outputRoot, "shared/index.md"),
  "---\ntitle: Shared packages\ndescription: Utilities shared by the three chain SDKs.\n---\n\n" +
    sharedEntries
      .map(
        (entry) =>
          `- [${entry.importName}](./${entry.section.slice("shared/".length)}/index.md)`,
      )
      .join("\n") +
    "\n",
);

const inventory = entries.map((entry) => {
  const module = project.children.find((child) => child.name === entry.section);

  return {
    package: entry.importName,
    section: entry.section,
    entryPoint: path.relative(repositoryRoot, entry.entryPoint),
    exports: module.children.map((child) => child.name).sort(),
  };
});
await writeFile(
  path.join(temporaryRoot, "inventory.json"),
  `${JSON.stringify(inventory, null, 2)}\n`,
);
console.log(
  `Generated ${markdownFiles.length + chains.length + 1} reference pages from ${entries.length} public entry points.`,
);

const allFiles = await readdir(outputRoot, {
  recursive: true,
});
for (const file of allFiles.filter((item) => item.endsWith(".md"))) {
  const filePath = path.join(outputRoot, file);
  const content = await readFile(filePath, "utf8");
  const resolved = resolveReferenceLinks(content, filePath);
  if (resolved !== content) {
    await writeFile(filePath, resolved);
  }
}
