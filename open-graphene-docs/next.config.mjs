import { createMDX } from "fumadocs-mdx/next";
import { createVanillaExtractPlugin } from "@vanilla-extract/next-plugin";

const withMDX = createMDX();
const withVanillaExtract = createVanillaExtractPlugin();

/** @type {import('next').NextConfig} */
const config = {
  output: "export",
  turbopack: {
    root: import.meta.dirname,
  },
  basePath: "/open-graphene",
  trailingSlash: true,
  reactStrictMode: true,
  images: {
    unoptimized: true,
  },
};

export default withVanillaExtract(withMDX(config));
