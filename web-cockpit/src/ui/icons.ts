export const ICON_NAMES = [
  "arrow-left", "arrow-up", "paperclip", "plus", "link",
  "chevron-down", "chevron-right", "x", "square", "folder", "sliders-horizontal", "settings",
] as const;

export type IconName = (typeof ICON_NAMES)[number];

export interface IconDefinition {
  readonly paths: readonly string[];
}

export const LUCIDE_ICONS: Readonly<Record<IconName, IconDefinition>> = {
  "arrow-left": { paths: ["m12 19-7-7 7-7", "M19 12H5"] },
  "arrow-up": { paths: ["M12 19V5", "m5 12 7-7 7 7"] },
  paperclip: {
    paths: ["m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48"],
  },
  plus: { paths: ["M5 12h14", "M12 5v14"] },
  link: {
    paths: [
      "M10 13a5 5 0 0 0 7.07 0l2-2a5 5 0 0 0-7.07-7.07l-1.72 1.71",
      "M14 11a5 5 0 0 0-7.07 0l-2 2a5 5 0 0 0 7.07 7.07l1.71-1.71",
    ],
  },
  "chevron-down": { paths: ["m6 9 6 6 6-6"] },
  "chevron-right": { paths: ["m9 18 6-6-6-6"] },
  x: { paths: ["m18 6-12 12", "m6 6 12 12"] },
  square: { paths: ["M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"] },
  folder: { paths: ["M4 5h6l2 2h8v12H4z"] },
  "sliders-horizontal": {
    paths: [
      "M21 4h-7", "M10 4H3", "M21 12h-9", "M8 12H3", "M21 20h-5", "M12 20H3",
      "M14 2v4", "M8 10v4", "M16 18v4",
    ],
  },
  settings: {
    paths: [
      "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.09a2 2 0 0 1-1-1.74v-.51a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z",
      "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
    ],
  },
};

export type IconSize = "sm" | "md" | "lg";

export interface IconOptions {
  readonly size?: IconSize;
}

export function renderIcon(
  document: Document,
  name: IconName,
  options: IconOptions = {},
): SVGSVGElement {
  const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  icon.classList.add("icon");
  if (options.size === "sm" || options.size === "lg") icon.classList.add(`icon--${options.size}`);
  icon.setAttribute("viewBox", "0 0 24 24");
  icon.setAttribute("fill", "none");
  icon.setAttribute("stroke", "currentColor");
  icon.setAttribute("stroke-width", "2");
  icon.setAttribute("stroke-linecap", "round");
  icon.setAttribute("stroke-linejoin", "round");
  icon.setAttribute("aria-hidden", "true");

  for (const d of LUCIDE_ICONS[name].paths) {
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute("d", d);
    icon.append(path);
  }
  return icon;
}
