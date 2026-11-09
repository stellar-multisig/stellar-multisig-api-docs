import { MDXRemote } from "next-mdx-remote/rsc";

export async function MDXRenderer({
  content,
}: {
  content: string;
}) {
  return <MDXRemote source={content} />;
}
