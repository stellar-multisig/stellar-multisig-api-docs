import fs from "fs/promises";
import path from "path";
import { MDXRenderer } from "@/components/MDXRenderer";

export default async function Page() {
  const content = await fs.readFile(
    path.join(process.cwd(), "src/app/docs/web-app-guide/page.mdx"),
    "utf-8"
  );
  return <MDXRenderer content={content} />;
}
