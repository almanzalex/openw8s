"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { AlertCircle, Loader2 } from "lucide-react";
import { HeroSearch } from "@/components/hero-search";
import { VramMatrix } from "@/components/vram-matrix";
import type { ModelProfile } from "@/lib/hf";

export function InspectClient() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const q = searchParams.get("q") ?? "";

  const [profile, setProfile] = useState<ModelProfile | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const runInspect = useCallback(async (repo: string) => {
    if (!repo.trim()) return;
    setLoading(true);
    setError(null);
    setProfile(null);
    try {
      const res = await fetch(`/api/inspect?repo=${encodeURIComponent(repo)}`);
      const data = await res.json();
      if (!res.ok) throw new Error(data.error ?? "Inspect failed");
      setProfile(data as ModelProfile);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Inspect failed");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (q) void runInspect(q);
  }, [q, runInspect]);

  return (
    <div className="space-y-8">
      <HeroSearch initial={q} />

      {loading && (
        <div className="flex items-center gap-3 border border-border bg-panel px-4 py-6 font-mono text-sm text-muted">
          <Loader2 className="h-4 w-4 animate-spin text-accent" />
          Fetching Hugging Face metadata…
        </div>
      )}

      {error && (
        <div className="flex items-start gap-3 border border-danger/40 bg-danger/10 px-4 py-4 font-mono text-sm text-danger">
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
          <div>
            <p className="font-medium">inspect failed</p>
            <p className="mt-1 text-danger/80">{error}</p>
            <button
              type="button"
              className="mt-3 text-xs underline underline-offset-2"
              onClick={() => router.push("/inspect")}
            >
              clear
            </button>
          </div>
        </div>
      )}

      {profile && !loading && <VramMatrix profile={profile} />}

      {!q && !loading && !error && !profile && (
        <div className="border border-dashed border-border bg-panel/40 px-4 py-10 text-center font-mono text-sm text-muted">
          Paste a Hugging Face repo id or URL to generate a VRAM matrix —
          same output as{" "}
          <code className="text-accent">openw8s inspect</code>.
          <div className="mt-4 flex flex-wrap justify-center gap-2">
            {[
              "Qwen/Qwen2.5-7B-Instruct",
              "meta-llama/Llama-3.1-8B-Instruct",
              "mistralai/Mistral-Nemo-Instruct-2407",
            ].map((example) => (
              <button
                key={example}
                type="button"
                onClick={() =>
                  router.push(`/inspect?q=${encodeURIComponent(example)}`)
                }
                className="border border-border bg-panel px-2.5 py-1 text-xs text-foreground transition hover:border-accent/40 hover:text-accent"
              >
                {example}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
