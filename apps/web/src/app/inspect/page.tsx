import { Suspense } from "react";
import { InspectClient } from "@/components/inspect-client";

export const metadata = {
  title: "Model Inspector — openw8s",
  description:
    "Paste a Hugging Face model URL and see VRAM / GPU requirements as an interactive table.",
};

export default function InspectPage() {
  return (
    <main className="grid-bg mx-auto max-w-6xl px-4 py-12 sm:px-6 sm:py-16">
      <p className="mb-3 font-mono text-xs uppercase tracking-[0.2em] text-accent">
        openw8s inspect
      </p>
      <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">
        Model Inspector
      </h1>
      <p className="mt-3 max-w-2xl text-muted">
        UI twin of the CLI. Resolves Hugging Face metadata and prints a VRAM
        matrix across quantizations and context lengths — no weight download.
      </p>

      <div className="mt-10">
        <Suspense
          fallback={
            <div className="border border-border bg-panel px-4 py-6 font-mono text-sm text-muted">
              Loading inspector…
            </div>
          }
        >
          <InspectClient />
        </Suspense>
      </div>
    </main>
  );
}
