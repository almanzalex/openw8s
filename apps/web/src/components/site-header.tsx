"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Terminal } from "lucide-react";

const NAV = [
  { href: "/", label: "Registry" },
  { href: "/inspect", label: "Inspector" },
];

export function SiteHeader() {
  const pathname = usePathname();

  return (
    <header className="sticky top-0 z-40 border-b border-border/80 bg-background/80 backdrop-blur-md">
      <div className="mx-auto flex h-14 max-w-6xl items-center justify-between px-4 sm:px-6">
        <Link href="/" className="group flex items-center gap-2.5 font-mono">
          <span className="flex h-7 w-7 items-center justify-center border border-accent/40 bg-accent/10 text-accent transition group-hover:bg-accent/20">
            <Terminal className="h-4 w-4" strokeWidth={2} />
          </span>
          <span className="text-sm font-semibold tracking-tight">
            openw8s
            <span className="ml-2 text-muted">v0.1</span>
          </span>
        </Link>

        <nav className="flex items-center gap-1 font-mono text-sm">
          {NAV.map((item) => {
            const active =
              item.href === "/"
                ? pathname === "/"
                : pathname.startsWith(item.href);
            return (
              <Link
                key={item.href}
                href={item.href}
                className={`px-3 py-1.5 transition ${
                  active
                    ? "bg-panel text-accent"
                    : "text-muted hover:bg-panel-2 hover:text-foreground"
                }`}
              >
                {item.label}
              </Link>
            );
          })}
          <a
            href="https://github.com/almanzalex/openw8s"
            target="_blank"
            rel="noopener noreferrer"
            className="ml-2 px-3 py-1.5 text-muted transition hover:text-foreground"
          >
            GitHub
          </a>
        </nav>
      </div>
    </header>
  );
}
