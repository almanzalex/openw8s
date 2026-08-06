import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  turbopack: {
    // Silence multi-lockfile root inference (repo root + apps/web).
    root: process.cwd(),
  },
};

export default nextConfig;
