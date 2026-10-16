import { source } from "@/lib/source";
import { createFromSource } from "fumadocs-core/search/server";

export const revalidate = false;

const referenceSections: Record<string, string> = {
  bitshares: "BitShares",
  swaplock: "Swaplock",
  rsquared: "R-Squared",
  shared: "Shared packages",
  typescript: "TypeScript",
  react: "React",
  bindings: "Bindings",
};
const structuralHeadings = new Set([
  "Parameters",
  "Returns",
  "Type Parameters",
  "Properties",
  "Methods",
  "Constructors",
  "Accessors",
  "Index Signatures",
  "Type Declaration",
  "Enumeration Members",
  "Implementation of",
  "Overrides",
  "Inherited from",
]);

export const { staticGET: GET } = createFromSource(source, {
  language: "english",
  buildIndex(page) {
    const isReference = page.slugs[0] === "reference";
    const data = page.data.structuredData;
    const breadcrumbs = isReference
      ? page.slugs.slice(1, 3).map((part) => referenceSections[part] ?? part)
      : undefined;

    // Index API names and member headings once. Repeated parameter tables and
    // type declarations make a full-text reference index unnecessarily large.
    const structuredData = isReference
      ? {
          headings: data.headings.filter((heading) => {
            return !structuralHeadings.has(heading.content);
          }),
          contents: [],
        }
      : data;

    return {
      id: page.url,
      title: page.data.title,
      description: page.data.description,
      breadcrumbs,
      url: page.url,
      structuredData,
    };
  },
});
