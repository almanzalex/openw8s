"use client";

import { useMemo, useState } from "react";
import { Check, Copy, Terminal } from "lucide-react";
import { SampleManifest, manifestToYaml } from "@/lib/manifests";

export function ManifestViewer({ manifest }: { manifest: SampleManifest }) {
  const yaml = useMemo(() => manifestToYaml(manifest), [manifest]);
  const [copiedYaml, setCopiedYaml] = useState(false);
  const [copiedCmd, setCopiedCmd] = useState(false);

  const lineage = manifest.metadata.lineage;
  const oneShot = `curl -fsSL https://raw.githubusercontent.com/almanzalex/openw8s/master/examples/${manifest.slug}.openw8s.yml -o .openw8s.yml && openw8s validate && openw8s run`;

  async function copy(text: string, which: "yaml" | "cmd") {
    await navigator.clipboard.writeText(text);
    if (which === "yaml") {
      setCopiedYaml(true);
      setTimeout(() => setCopiedYaml(false), 1600);
    } else {
      setCopiedCmd(true);
      setTimeout(() => setCopiedCmd(false), 1600);
    }
  }

  return (
    <div className="border border-border bg-panel">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border px-4 py-3">
        <div className="min-w-0">
          <p className="truncate font-mono text-sm font-medium text-foreground">
            {manifest.metadata.name}
          </p>
          <p className="truncate font-mono text-xs text-muted">
            {manifest.metadata.base_model} · {manifest.runtime.engine} ·{" "}
            {manifest.hardware.quantization} · {manifest.hardware.recommended_vram_gb}{" "}
            GB
            {lineage
              ? ` · ${lineage.kind}${lineage.parent ? ` → ${lineage.parent}` : ""}`
              : ""}
          </p>
        </div>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => copy(yaml, "yaml")}
            className="inline-flex items-center gap-1.5 border border-border bg-panel-2 px-2.5 py-1.5 font-mono text-xs text-muted transition hover:border-accent/40 hover:text-accent"
          >
            {copiedYaml ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            {copiedYaml ? "copied" : "copy yaml"}
          </button>
          <button
            type="button"
            onClick={() => copy(oneShot, "cmd")}
            className="inline-flex items-center gap-1.5 border border-accent/30 bg-accent/10 px-2.5 py-1.5 font-mono text-xs text-accent transition hover:bg-accent/20"
          >
            {copiedCmd ? <Check className="h-3.5 w-3.5" /> : <Terminal className="h-3.5 w-3.5" />}
            {copiedCmd ? "copied" : "copy one-shot"}
          </button>
        </div>
      </div>

      <pre className="terminal-scroll max-h-[320px] overflow-auto p-4 font-mono text-[12.5px] leading-relaxed text-foreground/90">
        {yaml.split("\n").map((line, i) => (
          <div key={i} className="whitespace-pre">
            <YamlLine line={line} />
          </div>
        ))}
      </pre>
    </div>
  );
}

function YamlLine({ line }: { line: string }) {
  if (!line.trim()) return <span>&nbsp;</span>;

  const comment = line.match(/^(\s*)(#.*)$/);
  if (comment) {
    return (
      <>
        <span>{comment[1]}</span>
        <span className="text-muted">{comment[2]}</span>
      </>
    );
  }

  const kv = line.match(/^(\s*)([\w-]+:)(\s*)(.*)$/);
  if (kv) {
    const [, indent, key, space, rest] = kv;
    return (
      <>
        <span>{indent}</span>
        <span className="text-accent">{key}</span>
        <span>{space}</span>
        <span className={rest.startsWith('"') || rest.startsWith(">") ? "text-warn" : ""}>
          {rest}
        </span>
      </>
    );
  }

  return <span>{line}</span>;
}
