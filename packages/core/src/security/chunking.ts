import { extname, join } from "node:path";
import { isVcsPath } from "./static";

// ── Types ──

export type Chunk = {
  file: string;
  lineRange: [number, number];
  content: string;
  index: number;
};

type FileChunk = Omit<Chunk, "index">;

// ── Constants ──

export const MAX_CHUNK_SIZE = 2000;
export const CHUNK_OVERLAP = 200;

/** Binary extensions to skip during chunking. */
const BINARY_EXTENSIONS = new Set([
  ".wasm",
  ".pyc",
  ".class",
  ".zip",
  ".tar",
  ".gz",
  ".png",
  ".jpg",
  ".jpeg",
  ".gif",
  ".ico",
  ".svg",
  ".woff",
  ".woff2",
  ".ttf",
  ".eot",
  ".pdf",
  ".exe",
  ".dll",
  ".so",
  ".dylib",
]);

// ── Chunking ──

/**
 * Read all text files in a skill directory and split into chunks.
 * Skips .git/, binary files, and files that fail UTF-8 decoding.
 */
export async function chunkSkillDir(dir: string): Promise<Chunk[]> {
  const glob = new Bun.Glob("**/*");
  const chunks: Chunk[] = [];
  let chunkIndex = 0;

  const relPaths: string[] = [];
  for await (const relPath of glob.scan({
    cwd: dir,
    onlyFiles: true,
    dot: true,
  })) {
    if (isVcsPath(relPath)) continue;
    const ext = extname(relPath).toLowerCase();
    if (BINARY_EXTENSIONS.has(ext)) continue;
    relPaths.push(relPath);
  }

  // Sort for deterministic ordering
  relPaths.sort();

  const allFileChunks: FileChunk[] = [];
  for (const relPath of relPaths) {
    const filePath = join(dir, relPath);
    let content: string;
    try {
      const bytes = await Bun.file(filePath).arrayBuffer();
      content = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    } catch {
      continue; // Skip binary/non-UTF-8 files
    }

    allFileChunks.push(...splitIntoChunks(content, relPath));
  }

  // Add overlap chunks to catch payloads split across boundaries
  const withOverlaps = addOverlapChunks(allFileChunks);

  for (const fc of withOverlaps) {
    chunks.push({ ...fc, index: chunkIndex++ });
  }

  return chunks;
}

function splitIntoChunks(content: string, file: string): FileChunk[] {
  if (content.length <= MAX_CHUNK_SIZE) {
    const lineCount = content.split("\n").length;
    return [{ file, lineRange: [1, lineCount], content }];
  }

  const paragraphs = content.split(/\n\n+/);
  const result: FileChunk[] = [];
  let currentContent = "";
  let currentStartLine = 1;

  for (const para of paragraphs) {
    const separator = currentContent ? "\n\n" : "";
    const combined = currentContent + separator + para;

    if (combined.length > MAX_CHUNK_SIZE && currentContent) {
      // Flush current chunk
      const endLine = currentStartLine + countLines(currentContent) - 1;
      result.push({
        file,
        lineRange: [currentStartLine, endLine],
        content: currentContent,
      });
      currentStartLine = endLine + 1; // +1 for the blank line between paragraphs
      currentContent = para;
    } else {
      currentContent = combined;
    }
  }

  // Flush last chunk
  if (currentContent) {
    const endLine = currentStartLine + countLines(currentContent) - 1;
    result.push({
      file,
      lineRange: [currentStartLine, endLine],
      content: currentContent,
    });
  }

  // Post-process: split any over-sized chunks further
  const final: FileChunk[] = [];
  for (const chunk of result) {
    if (chunk.content.length <= MAX_CHUNK_SIZE) {
      final.push(chunk);
    } else {
      final.push(...splitBySentence(chunk));
    }
  }

  return final;
}

function splitBySentence(chunk: FileChunk): FileChunk[] {
  const { content, file } = chunk;
  let startLine = chunk.lineRange[0];

  // Try sentence boundaries: ". " followed by uppercase or newline
  const sentences = content.split(/(?<=\.\s)(?=[A-Z\n])/);
  const result: FileChunk[] = [];
  let current = "";

  for (const sentence of sentences) {
    const combined = current + sentence;
    if (combined.length > MAX_CHUNK_SIZE && current) {
      const endLine = startLine + countLines(current) - 1;
      result.push({ file, lineRange: [startLine, endLine], content: current });
      startLine = endLine + 1;
      current = sentence;
    } else {
      current = combined;
    }
  }

  if (current) {
    const endLine = startLine + countLines(current) - 1;
    result.push({ file, lineRange: [startLine, endLine], content: current });
  }

  // Hard split anything still over limit
  const final: FileChunk[] = [];
  for (const c of result) {
    if (c.content.length <= MAX_CHUNK_SIZE) {
      final.push(c);
    } else {
      final.push(...hardSplit(c));
    }
  }

  return final;
}

function hardSplit(chunk: FileChunk): FileChunk[] {
  const { content, file } = chunk;
  let startLine = chunk.lineRange[0];
  const result: FileChunk[] = [];

  for (let i = 0; i < content.length; i += MAX_CHUNK_SIZE) {
    const slice = content.slice(i, i + MAX_CHUNK_SIZE);
    const endLine = startLine + countLines(slice) - 1;
    result.push({ file, lineRange: [startLine, endLine], content: slice });
    startLine = endLine;
  }

  return result;
}

/**
 * Generate overlap chunks that span the boundaries between adjacent chunks.
 * Each overlap chunk takes the last CHUNK_OVERLAP chars of chunk N and the
 * first CHUNK_OVERLAP chars of chunk N+1, catching payloads that an attacker
 * might split across a predictable boundary.
 */
function addOverlapChunks(chunks: FileChunk[]): FileChunk[] {
  if (chunks.length < 2) return chunks;

  const result: FileChunk[] = [];
  for (let i = 0; i < chunks.length; i++) {
    // biome-ignore lint/style/noNonNullAssertion: i is within bounds
    result.push(chunks[i]!);

    if (i < chunks.length - 1) {
      // biome-ignore lint/style/noNonNullAssertion: i and i+1 are within bounds
      const current = chunks[i]!;
      // biome-ignore lint/style/noNonNullAssertion: i+1 is within bounds
      const next = chunks[i + 1]!;

      // Only overlap chunks from the same file
      if (current.file !== next.file) continue;

      const tail = current.content.slice(-CHUNK_OVERLAP);
      const head = next.content.slice(0, CHUNK_OVERLAP);
      const overlapContent = `${tail}\n\n${head}`;

      // Skip if the overlap is trivially small
      if (overlapContent.length < 50) continue;

      result.push({
        file: current.file,
        lineRange: [current.lineRange[1], next.lineRange[0]],
        content: overlapContent,
      });
    }
  }

  return result;
}

function countLines(text: string): number {
  if (!text) return 1;
  let count = 1;
  for (let i = 0; i < text.length; i++) {
    if (text[i] === "\n") count++;
  }
  return count;
}
