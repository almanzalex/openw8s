"use client";

import { SampleManifest } from "@/lib/manifests";

interface Node {
  id: string;
  label: string;
  kind: "base" | "env";
}

interface Edge {
  from: string;
  to: string;
  kind: string;
}

function buildGraph(manifests: SampleManifest[]): { nodes: Node[]; edges: Edge[] } {
  const nodes = new Map<string, Node>();
  const edges: Edge[] = [];

  for (const m of manifests) {
    const envId = `env:${m.slug}`;
    nodes.set(envId, {
      id: envId,
      label: m.metadata.name,
      kind: "env",
    });

    const baseId = `model:${m.metadata.base_model}`;
    if (!nodes.has(baseId)) {
      nodes.set(baseId, {
        id: baseId,
        label: m.metadata.base_model,
        kind: "base",
      });
    }
    edges.push({ from: baseId, to: envId, kind: "packages" });

    const lineage = m.metadata.lineage;
    if (lineage?.parent) {
      const parentId = `model:${lineage.parent}`;
      if (!nodes.has(parentId)) {
        nodes.set(parentId, {
          id: parentId,
          label: lineage.parent,
          kind: "base",
        });
      }
      edges.push({
        from: parentId,
        to: baseId,
        kind: lineage.kind,
      });
    }
  }

  return { nodes: [...nodes.values()], edges };
}

export function LineageGraph({ manifests }: { manifests: SampleManifest[] }) {
  const { nodes, edges } = buildGraph(manifests);
  const bases = nodes.filter((n) => n.kind === "base");
  const envs = nodes.filter((n) => n.kind === "env");

  return (
    <div className="border border-border bg-panel">
      <div className="border-b border-border px-4 py-3">
        <h3 className="font-mono text-xs uppercase tracking-[0.2em] text-muted">
          Lineage graph
        </h3>
        <p className="mt-1 font-mono text-xs text-muted">
          {bases.length} models · {envs.length} environments · {edges.length} edges
        </p>
      </div>

      <div className="grid gap-6 p-4 lg:grid-cols-2">
        <div>
          <p className="mb-2 font-mono text-[11px] uppercase tracking-widest text-accent">
            Models
          </p>
          <ul className="space-y-1.5 font-mono text-xs">
            {bases.map((n) => (
              <li
                key={n.id}
                className="border border-border bg-panel-2 px-2.5 py-1.5 text-foreground"
              >
                {n.label}
              </li>
            ))}
          </ul>
        </div>

        <div>
          <p className="mb-2 font-mono text-[11px] uppercase tracking-widest text-accent">
            Environments
          </p>
          <ul className="space-y-1.5 font-mono text-xs">
            {envs.map((n) => (
              <li
                key={n.id}
                className="border border-border/80 bg-accent/5 px-2.5 py-1.5 text-foreground"
              >
                {n.label}
              </li>
            ))}
          </ul>
        </div>
      </div>

      <div className="border-t border-border px-4 py-3">
        <p className="mb-2 font-mono text-[11px] uppercase tracking-widest text-muted">
          Edges
        </p>
        <ul className="max-h-48 space-y-1 overflow-auto font-mono text-[11px] text-muted terminal-scroll">
          {edges.map((e, i) => (
            <li key={`${e.from}-${e.to}-${i}`}>
              <span className="text-foreground">{e.from.replace(/^(model|env):/, "")}</span>
              <span className="mx-2 text-accent">{e.kind}</span>
              <span className="text-foreground">{e.to.replace(/^(model|env):/, "")}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
