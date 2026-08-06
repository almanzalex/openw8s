import { NextRequest, NextResponse } from "next/server";
import { inspectModel } from "@/lib/hf";

export async function GET(request: NextRequest) {
  const repo = request.nextUrl.searchParams.get("repo");
  if (!repo) {
    return NextResponse.json({ error: "Missing `repo` query parameter" }, { status: 400 });
  }

  try {
    const profile = await inspectModel(repo);
    return NextResponse.json(profile);
  } catch (err) {
    const message = err instanceof Error ? err.message : "Inspect failed";
    const status = message.includes("not found")
      ? 404
      : message.includes("private") || message.includes("gated")
        ? 403
        : 502;
    return NextResponse.json({ error: message }, { status });
  }
}
