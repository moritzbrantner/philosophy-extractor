// Regenerates document-search-batch.jsonl with document-search's real exporter.
// Usage (see README.md): bun run <this file> <document-search src root at the pinned commit>
// It imports document-search's public import path and corpus-core exporter unchanged;
// nothing here re-implements the export.
import { resolve } from "node:path";

const root = process.argv[2];
if (!root) throw new Error("usage: bun run generate-document-search-batch.ts <document-search checkout root>");

const { parseUploadedDocument } = await import(resolve(root, "src/domain/documentImport.ts"));
const { extractHtmlDocument } = await import(resolve(root, "src/domain/htmlExtraction.ts"));
const { createCorpusCoreBatch, serializeCorpusCoreBatch } = await import(
  resolve(root, "src/storage/corpusCoreExport.ts")
);

const IMPORTED_AT = "2026-10-01T00:00:00.000Z";

function uploaded(id: string, name: string, content: string) {
  return extractHtmlDocument({
    ...parseUploadedDocument({ name, content, importedAt: IMPORTED_AT }),
    id,
  });
}

const documents = [
  // Markdown upload with multi-byte text: scalar ranges differ from UTF-8 byte ranges.
  uploaded(
    "phil-meditation",
    "meditation.md",
    "# Méditation 𝄞\n\nI think, therefore I am — cogito 😀 ergo sum.\n\nDoubt is the beginning of wisdom.\n",
  ),
  // Plain-text upload with a soft line break folded into one paragraph.
  uploaded(
    "phil-ethics",
    "ethics.txt",
    "Virtue is a mean\nbetween two vices.\n\nHappiness is activity in accordance with virtue.\n",
  ),
  // URL-addressed HTML stored before originals were kept: the asset carries a hash but no payload.
  (() => {
    const document = extractHtmlDocument({
      id: "phil-web",
      title: "Remote Essay",
      sourceUri: "https://example.test/essay",
      html: "<h2>On Truth</h2><p>Truth is correspondence ✓.</p>",
      importedAt: IMPORTED_AT,
    });
    const legacy = { ...document } as typeof document & { original?: unknown };
    delete legacy.original;
    return legacy;
  })(),
];

process.stdout.write(serializeCorpusCoreBatch(await createCorpusCoreBatch(documents, "git:28463de")));
