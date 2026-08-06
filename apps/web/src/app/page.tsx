import Link from "next/link";
import { ArrowRight, Cpu, FileCode2, GitBranch } from "lucide-react";
import { HeroSearch } from "@/components/hero-search";
import { LineageGraph } from "@/components/lineage-graph";
import { ManifestViewer } from "@/components/manifest-viewer";
import { SAMPLE_MANIFESTS } from "@/lib/manifests";

export default function HomePage() {
  return (
    <main className="grid-bg">
      <section className="mx-auto max-w-6xl px-4 pb-16 pt-16 sm:px-6 sm:pt-24">
        <p className="mb-4 font-mono text-xs uppercase tracking-[0.2em] text-accent">
          openw8s · open weights spec
        </p>
        <h1 className="max-w-3xl text-3xl font-semibold tracking-tight text-foreground sm:text-5xl sm:leading-[1.1]">
          The Open Weights Spec & Environment Registry
        </h1>
        <p className="mt-5 max-w-2xl text-base leading-relaxed text-muted sm:text-lg">
          Standardize how open-weight models are profiled, packaged, and run.
          One{" "}
          <code className="font-mono text-accent">.openw8s.yml</code> file for
          VRAM requirements, runtime commands, and lineage.
        </p>

        <div className="mt-10">
          <HeroSearch />
        </div>

        <div className="mt-8 flex flex-wrap gap-3 font-mono text-xs text-muted">
          <span className="border border-border bg-panel px-2.5 py-1">
            $ openw8s inspect org/model
          </span>
          <span className="border border-border bg-panel px-2.5 py-1">
            $ openw8s init
          </span>
          <span className="border border-border bg-panel px-2.5 py-1">
            $ openw8s run
          </span>
        </div>
      </section>

      <section className="border-y border-border bg-panel/40">
        <div className="mx-auto grid max-w-6xl gap-px bg-border sm:grid-cols-3">
          {[
            {
              icon: Cpu,
              title: "VRAM matrix",
              body: "Estimate FP16 / INT8 / INT4 footprints across 4k–32k context without downloading weights.",
            },
            {
              icon: FileCode2,
              title: "Manifest standard",
              body: "A single YAML contract for hardware, quantization, runtime engine, and eval scores.",
            },
            {
              icon: GitBranch,
              title: "Lineage tracking",
              body: "Optional parent_branch fields map fine-tunes back to their base open-weight models.",
            },
          ].map(({ icon: Icon, title, body }) => (
            <div key={title} className="bg-background px-6 py-8">
              <Icon className="mb-3 h-5 w-5 text-accent" strokeWidth={1.75} />
              <h2 className="font-mono text-sm font-medium text-foreground">
                {title}
              </h2>
              <p className="mt-2 text-sm leading-relaxed text-muted">{body}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="mx-auto max-w-6xl px-4 py-16 sm:px-6">
        <div className="mb-8 flex flex-wrap items-end justify-between gap-4">
          <div>
            <h2 className="font-mono text-xs uppercase tracking-[0.2em] text-muted">
              Community manifests
            </h2>
            <p className="mt-2 text-xl font-semibold tracking-tight">
              Featured .openw8s.yml environments
            </p>
            <p className="mt-1 font-mono text-xs text-muted">
              Seed catalog from /examples · also at /m/&lt;slug&gt;.yml
            </p>
          </div>
          <Link
            href="/inspect"
            className="inline-flex items-center gap-1.5 font-mono text-sm text-accent transition hover:underline"
          >
            Open inspector <ArrowRight className="h-4 w-4" />
          </Link>
        </div>

        <div className="mb-10">
          <LineageGraph manifests={SAMPLE_MANIFESTS} />
        </div>

        <div className="space-y-6">
          {SAMPLE_MANIFESTS.map((m) => (
            <ManifestViewer key={m.slug} manifest={m} />
          ))}
        </div>
      </section>

      <footer className="border-t border-border py-8 text-center font-mono text-xs text-muted">
        openw8s v0.1 · Apache-2.0 · Spec + CLI + Registry
      </footer>
    </main>
  );
}
