import { createGetUrl } from "fumadocs-core/source";

export const appName = "Open Graphene";
export const basePath = "/open-graphene";
export const docsRoute = "/docs";
export const docsContentRoute = "/llms.mdx/docs";

export const gitConfig = {
  user: "open-graphene",
  repo: "open-graphene",
  branch: "main",
};

const getContentUrl = createGetUrl(docsContentRoute);

export function getPageMarkdownUrl(page: { slugs: string[]; locale?: string }) {
  const segments = [...page.slugs, "content.md"];

  return {
    segments,
    url: `${basePath}${getContentUrl(segments, page.locale)}`,
  };
}
