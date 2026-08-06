"use client";

import { ModelProfile } from "@/lib/hf";
import {
  CONTEXT_LENGTHS,
  QUANTIZATIONS,
  estimateTotalVramGb,
} from "@/lib/vram";

function ctxLabel(n: number): string {
  if (n >= 1024) return `${n / 1024}k ctx`;
  return `${n} ctx`;
}

export function VramMatrix({ profile }: { profile: ModelProfile }) {
  return (
    <div className="space-y-6">
      <div className="overflow-x-auto border border-border bg-panel">
        <table className="w-full min-w-[420px] font-mono text-sm">
          <tbody>
            {[
              ["Model Name", profile.modelName],
              ["Repo ID", profile.repoId],
              ["Total Params", profile.paramsLabel],
              ["File Format", profile.fileFormat],
              ...(profile.arch.hiddenSize
                ? [["Hidden Size", String(profile.arch.hiddenSize)]]
                : []),
              ...(profile.arch.numHiddenLayers
                ? [["Layers", String(profile.arch.numHiddenLayers)]]
                : []),
            ].map(([k, v]) => (
              <tr key={k} className="border-b border-border last:border-0">
                <th className="w-40 bg-panel-2 px-4 py-2.5 text-left font-medium text-muted">
                  {k}
                </th>
                <td className="px-4 py-2.5 text-foreground">{v}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div>
        <h3 className="mb-3 font-mono text-xs uppercase tracking-widest text-muted">
          Estimated VRAM — weights + KV cache + 10% overhead
        </h3>
        <div className="overflow-x-auto border border-border bg-panel">
          <table className="w-full min-w-[520px] font-mono text-sm">
            <thead>
              <tr className="border-b border-border bg-panel-2 text-left text-accent">
                <th className="px-4 py-2.5 font-medium">Quantization</th>
                {CONTEXT_LENGTHS.map((c) => (
                  <th key={c} className="px-4 py-2.5 font-medium">
                    {ctxLabel(c)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {QUANTIZATIONS.map((q) => (
                <tr key={q} className="border-b border-border last:border-0">
                  <td className="px-4 py-2.5 text-muted">{q}</td>
                  {CONTEXT_LENGTHS.map((ctx) => {
                    const gb = estimateTotalVramGb(
                      profile.paramsBillions,
                      q,
                      ctx,
                      profile.arch,
                    );
                    return (
                      <td key={ctx} className="px-4 py-2.5 tabular-nums">
                        <span
                          className={
                            gb > 48
                              ? "text-danger"
                              : gb > 24
                                ? "text-warn"
                                : "text-accent"
                          }
                        >
                          {gb.toFixed(1)} GB
                        </span>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="mt-3 font-mono text-xs text-muted">
          Estimates are approximate. Add activation / framework overhead for
          production serving.
        </p>
      </div>
    </div>
  );
}
