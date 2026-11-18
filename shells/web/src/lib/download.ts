/**
 * Hand a blob to the browser as a file download.
 *
 * @param blob - File contents.
 * @param filename - Suggested file name including extension.
 */
export function downloadBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.rel = "noopener";
  document.body.appendChild(a);
  a.click();
  a.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 30000);
}

/**
 * File-name-safe version of a user-visible name.
 *
 * @param name - Free text such as a recipe name.
 * @returns The name with path separators and control characters replaced, never empty.
 */
export function safeFileName(name: string): string {
  // eslint-disable-next-line no-control-regex -- control characters are exactly what must be stripped
  const cleaned = name.replace(/[\\/:*?"<>|\u0000-\u001f]+/g, "-").trim();
  return cleaned === "" ? "untitled" : cleaned.slice(0, 120);
}

/**
 * Open the system file picker for one file.
 *
 * @param accept - `accept` attribute value, e.g. `.cube` or `application/json,.json`.
 * @param onFile - Called with the chosen file.
 */
export function pickFile(accept: string, onFile: (file: File) => void): void {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = accept;
  input.addEventListener("change", () => {
    const f = input.files?.[0];
    if (f) onFile(f);
  });
  input.click();
}
