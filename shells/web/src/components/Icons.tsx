import type { JSX } from "solid-js";

interface IconProps {
  /** Pixel size of the square icon. */
  size?: number;
}

function Svg(props: IconProps & { children: JSX.Element }): JSX.Element {
  return (
    <svg
      width={props.size ?? 16}
      height={props.size ?? 16}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      {props.children}
    </svg>
  );
}

/** Undo arrow. */
export const UndoIcon = (p: IconProps): JSX.Element => (
  <Svg {...p}>
    <path d="M6 3 2.5 6.5 6 10" />
    <path d="M3 6.5h6.5a4 4 0 0 1 0 8H7" />
  </Svg>
);
/** Redo arrow. */
export const RedoIcon = (p: IconProps): JSX.Element => (
  <Svg {...p}>
    <path d="M10 3l3.5 3.5L10 10" />
    <path d="M13 6.5H6.5a4 4 0 0 0 0 8H9" />
  </Svg>
);
/** Chevron pointing right; rotate with CSS for other directions. */
export const ChevronIcon = (p: IconProps): JSX.Element => (
  <Svg {...p}>
    <path d="M6 3.5 10.5 8 6 12.5" />
  </Svg>
);
/** Up arrow (prompt history). */
export const ArrowUpIcon = (p: IconProps): JSX.Element => (
  <Svg {...p}>
    <path d="M8 13V3" />
    <path d="M3.5 7.5 8 3l4.5 4.5" />
  </Svg>
);
/** Information glyph (fallback banner). */
export const InfoIcon = (p: IconProps): JSX.Element => (
  <Svg {...p}>
    <circle cx="8" cy="8" r="6" />
    <path d="M8 7.2V11" />
    <path d="M8 4.9v.1" />
  </Svg>
);
/** Check mark (legal). */
export const CheckIcon = (p: IconProps): JSX.Element => (
  <Svg {...p}>
    <path d="m3 8.5 3.2 3L13 4.5" />
  </Svg>
);
/** Warning triangle (illegal). */
export const WarnIcon = (p: IconProps): JSX.Element => (
  <Svg {...p}>
    <path d="M8 2.5 14 13H2L8 2.5Z" />
    <path d="M8 6.5v3" />
    <path d="M8 11.4v.1" />
  </Svg>
);
/** Close cross. */
export const CloseIcon = (p: IconProps): JSX.Element => (
  <Svg {...p}>
    <path d="m4 4 8 8M12 4l-8 8" />
  </Svg>
);
/** Half-filled circle (theme toggle). */
export const ThemeIcon = (p: IconProps): JSX.Element => (
  <Svg {...p}>
    <circle cx="8" cy="8" r="5.5" />
    <path d="M8 2.5v11a5.5 5.5 0 0 0 0-11Z" fill="currentColor" />
  </Svg>
);
/** Dice-like glyph for "Surprise me". */
export const ShuffleIcon = (p: IconProps): JSX.Element => (
  <Svg {...p}>
    <rect x="2.5" y="2.5" width="11" height="11" rx="2" />
    <path d="M5.6 5.6v.1M10.4 5.6v.1M8 8v.1M5.6 10.4v.1M10.4 10.4v.1" />
  </Svg>
);
/** Folder / open glyph. */
export const OpenIcon = (p: IconProps): JSX.Element => (
  <Svg {...p}>
    <path d="M2 12.5V4.5a1 1 0 0 1 1-1h3l1.5 1.5H13a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1Z" />
  </Svg>
);
