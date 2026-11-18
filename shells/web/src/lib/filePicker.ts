/** `accept` list for image import. */
export const IMAGE_ACCEPT =
  "image/jpeg,image/png,image/webp,image/heic,image/heif,.jpg,.jpeg,.png,.webp,.heic,.heif";

/**
 * Open the system file picker for one image.
 *
 * @param onFile - Called with the chosen file.
 */
export function pickImage(onFile: (file: File) => void): void {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = IMAGE_ACCEPT;
  input.addEventListener("change", () => {
    const f = input.files?.[0];
    if (f) onFile(f);
  });
  input.click();
}
