import { llms, loader } from "fumadocs-core/source";
import { basePath, docsRoute } from "./shared";
import { defineDocs } from "fumadocs-mdx/macro";
import { metaSchema, pageSchema } from "fumadocs-core/source/schema";

const docs = defineDocs({
  dir: "content/docs",
  docs: {
    schema: pageSchema,
    postprocess: {
      includeProcessedMarkdown: true,
    },
  },
  meta: {
    schema: metaSchema,
  },
});

// See https://fumadocs.dev/docs/headless/source-api for more info
export const source = loader({
  baseUrl: docsRoute,
  source: docs.toFumadocsSource(),
  plugins: [],
  pageTree: {
    // Symbol pages are reached through catalogs and search. Do not serialize
    // thousands of hidden entries into the navigation of every static page.
    generateFallback: false,
  },
});

export const docsLlms = llms(source, {
  renderPage: async (page) => `# ${page.data.title} (${basePath}${page.url})

${await page.data.getText("processed")}`,
});
