import { source } from "@/lib/source";
import { basePath } from "@/lib/shared";

export const revalidate = false;

export function GET() {
  const pages = source.getPages();
  const links = pages.map((page) => {
    const url = `https://open-graphene.github.io${basePath}${page.url}/`;

    return `- [${page.data.title}](${url}): ${page.data.description ?? ""}`;
  });

  return new Response(`# Open Graphene\n\n${links.join("\n")}\n`);
}
