"use client";

import { FormEvent, useState } from "react";
import { useRouter } from "next/navigation";
import { Search } from "lucide-react";

export function HeroSearch({ initial = "" }: { initial?: string }) {
  const router = useRouter();
  const [value, setValue] = useState(initial);

  function onSubmit(e: FormEvent) {
    e.preventDefault();
    const q = value.trim();
    if (!q) return;
    router.push(`/inspect?q=${encodeURIComponent(q)}`);
  }

  return (
    <form onSubmit={onSubmit} className="w-full max-w-2xl">
      <label className="mb-2 block font-mono text-xs uppercase tracking-widest text-muted">
        Inspect a Hugging Face model
      </label>
      <div className="flex border border-border bg-panel focus-within:border-accent/50">
        <div className="flex items-center pl-3 text-muted">
          <Search className="h-4 w-4" />
        </div>
        <input
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder="Qwen/Qwen2.5-7B-Instruct or huggingface.co/…"
          className="min-w-0 flex-1 bg-transparent px-3 py-3 font-mono text-sm text-foreground outline-none placeholder:text-muted/60"
          spellCheck={false}
          autoComplete="off"
        />
        <button
          type="submit"
          className="border-l border-border bg-accent/15 px-4 font-mono text-sm font-medium text-accent transition hover:bg-accent/25"
        >
          inspect
        </button>
      </div>
    </form>
  );
}
