import Link from "next/link";

export default function HomePage() {
  return (
    <main className="flex flex-1 flex-col items-center justify-center gap-6 px-6 py-24 text-center">
      <p className="text-sm font-medium text-fd-muted-foreground">
        OPEN GRAPHENE DOCUMENTATION
      </p>
      <h1 className="text-4xl font-semibold tracking-tight sm:text-6xl">
        Build on Graphene.
      </h1>
      <p className="max-w-xl text-lg text-fd-muted-foreground">
        Protocol specifications, generated bindings, and SDK tools for
        Graphene-based blockchains.
      </p>
      <Link
        href="/docs/"
        className="rounded-lg bg-fd-primary px-6 py-3 font-medium text-fd-primary-foreground"
      >
        Read the documentation
      </Link>
    </main>
  );
}
