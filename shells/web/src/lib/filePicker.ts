import { MEDIA_ACCEPT } from "../video/model";

/** `accept` list for image import. */
export const IMAGE_ACCEPT =
  "image/jpeg,image/png,image/webp,image/heic,image/heif,.jpg,.jpeg,.png,.webp,.heic,.heif";

/**
 * Open the system file picker for one image.
 *
 * @param onFile - Called with the chosen file.
 * @param accept - `accept` attribute value; images only by default.
 */
export function pickImage(onFile: (file: File) => void, accept: string = IMAGE_ACCEPT): void {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = accept;
  input.addEventListener("change", () => {
    const f = input.files?.[0];
    if (f) onFile(f);
  });
  input.click();
}

/**
 * Open the system file picker for one image or video.
 *
 * @param onFile - Called with the chosen file.
 */
export function pickMedia(onFile: (file: File) => void): void {
  pickImage(onFile, MEDIA_ACCEPT);
}
