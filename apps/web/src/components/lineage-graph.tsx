"use client";

import { SampleManifest } from "@/lib/manifests";

interface GraphNode {
  id: string;
  label: string;
  kind: "model" | "env";
  x: number;
  y: number;
}

interface GraphEdge {
  from: string;
  to: string;
  kind: string;
}

function layout(manifests: SampleManifest[]): {
  nodes: GraphNode[];
  edges: GraphEdge[];
  width: number;
  height: number;
} {
  const modelIds: string[] = [];
  const envIds: string[] = [];
  const edges: GraphEdge[] = [];
  const label = new Map<string, string>();

  for (const m of manifests) {
    const baseId = `model:${m.metadata.base_model}`;
    const envId = `env:${m.slug}`;
    label.set(baseId, m.metadata.base_model);
    label.set(envId, m.metadata.name);

    if (!modelIds.includes(baseId)) modelIds.push(baseId);
    if (!envIds.includes(envId)) envIds.push(envId);
    edges.push({ from: baseId, to: envId, kind: "packages" });

    const parent = m.metadata.lineage?.parent;
    const kind = m.metadata.lineage?.kind ?? "base";
    if (parent) {
      const parentId = `model:${parent}`;
      label.set(parentId, parent);
      if (!modelIds.includes(parentId)) modelIds.push(parentId);
      edges.push({ from: parentId, to: baseId, kind });
    }
  }

  const colGap = 280;
  const rowGap = 56;
  const leftX = 40;
  const rightX = leftX + colGap;
  const top = 36;

  const nodes: GraphNode[] = [];
  modelIds.forEach((id, i) => {
    nodes.push({
      id,
      label: shortLabel(label.get(id) ?? id),
      kind: "model",
      x: leftX,
      y: top + i * rowGap,
    });
  });
  envIds.forEach((id, i) => {
    nodes.push({
      id,
      label: shortLabel(label.get(id) ?? id),
      kind: "env",
      x: rightX,
      y: top + i * rowGap,
    });
  });

  const height = Math.max(modelIds.length, envIds.length) * rowGap + top + 24;
  const width = rightX + 220;
  return { nodes, edges, width, height };
}

function shortLabel(s: string): string {
  const bare = s.includes("/") ? (s.split("/").pop() ?? s) : s;
  return bare.length > 34 ? `${bare.slice(0, 32)}…` : bare;
}

export function LineageGraph({ manifests }: { manifests: SampleManifest[] }) {
  const { nodes, edges, width, height } = layout(manifests);
  const byId = new Map(nodes.map((n) => [n.id, n]));

  return (
    <div className="border border-border bg-panel">
      <div className="border-b border-border px-4 py-3">
        <h3 className="font-mono text-xs uppercase tracking-[0.2em] text-muted">
          Lineage graph
        </h3>
        <p className="mt-1 font-mono text-xs text-muted">
          models → environments · hover edges for relationship kind
        </p>
      </div>

      <div className="overflow-x-auto p-2 terminal-scroll">
        <svg
          width={width}
          height={Math.max(height, 180)}
          viewBox={`0 0 ${width} ${Math.max(height, 180)}`}
          role="img"
          aria-label="Model lineage graph"
          className="min-w-full"
        >
          <defs>
            <marker
              id="arrow"
              viewBox="0 0 10 10"
              refX="9"
              refY="5"
              markerWidth="6"
              markerHeight="6"
              orient="auto-start-reverse"
            >
              <path d="M 0 0 L 10 5 L 0 10 z" fill="#3dde9a" />
            </marker>
          </defs>

          {edges.map((e, i) => {
            const a = byId.get(e.from);
            const b = byId.get(e.to);
            if (!a || !b) return null;
            const x1 = a.x + 180;
            const y1 = a.y + 14;
            const x2 = b.x;
            const y2 = b.y + 14;
            const mx = (x1 + x2) / 2;
            return (
              <g key={`${e.from}-${e.to}-${i}`}>
                <path
                  d={`M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`}
                  stroke="#3dde9a"
                  strokeOpacity="0.55"
                  strokeWidth="1.5"
                  fill="none"
                  markerEnd="url(#arrow)"
                >
                  <title>{e.kind}</title>
                </path>
                <text
                  x={mx}
                  y={(y1 + y2) / 2 - 6}
                  textAnchor="middle"
                  className="fill-[color:var(--muted)]"
                  style={{ fontSize: 9, fontFamily: "var(--font-ibm-mono), monospace" }}
                >
                  {e.kind}
                </text>
              </g>
            );
          })}

          {nodes.map((n) => (
            <g key={n.id} transform={`translate(${n.x}, ${n.y})`}>
              <rect
                width="180"
                height="28"
                rx="0"
                fill={n.kind === "env" ? "rgba(61,222,154,0.08)" : "#121820"}
                stroke={n.kind === "env" ? "rgba(61,222,154,0.45)" : "#1e2833"}
              />
              <text
                x="8"
                y="18"
                style={{
                  fontSize: 10,
                  fontFamily: "var(--font-ibm-mono), monospace",
                  fill: n.kind === "env" ? "#3dde9a" : "#e8edf2",
                }}
              >
                {n.label}
              </text>
              <title>{n.id.replace(/^(model|env):/, "")}</title>
            </g>
          ))}
        </svg>
      </div>
    </div>
  );
}
