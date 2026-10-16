import type { Metadata } from "next";
import { Provider } from "@/components/provider";
import "./theme.css";
import "./global.css";

export const metadata: Metadata = {
  title: {
    default: "Open Graphene",
    template: "%s | Open Graphene",
  },
  description: "SDK tooling for Graphene-based blockchains.",
};

export default function Layout(props: LayoutProps<"/">) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className="flex flex-col min-h-screen">
        <Provider>{props.children}</Provider>
      </body>
    </html>
  );
}
