/**
 * A small app "icon". When the OS can give us the app's real icon (resolved
 * from its stored executable / bundle path) we render that; otherwise we fall
 * back to a deterministic colored chip with the first letter of the app name.
 *
 * Real icons are fetched once per app key and cached for the session (a shared
 * promise cache), so a list of rows doesn't refetch or repaint the same icon.
 */

import { useEffect, useState } from "react";
import { getAppIcon } from "../lib/api";

const CHIP_COLORS = [
  "#2DD4BF",
  "#0EA5A0",
  "#34D399",
  "#F59E0B",
  "#F87171",
  "#8B5CF6",
  "#60A5FA",
  "#F472B6",
];

function hashColor(key: string): string {
  let h = 0;
  for (let i = 0; i < key.length; i++) {
    h = (h * 31 + key.charCodeAt(i)) >>> 0;
  }
  return CHIP_COLORS[h % CHIP_COLORS.length];
}

// appKey -> data URL promise. Shared so lists do not refetch while a real icon
// is loading; misses are evicted so a later collector tick can fill exe_path.
const iconCache = new Map<string, Promise<string>>();

function loadIcon(appKey: string, force = false): Promise<string> {
  if (force) iconCache.delete(appKey);
  const cached = iconCache.get(appKey);
  if (cached) return cached;
  const p = getAppIcon(appKey)
    .then((icon) => {
      if (!icon || icon.width <= 0 || icon.height <= 0) return "";
      const canvas = document.createElement("canvas");
      canvas.width = icon.width;
      canvas.height = icon.height;
      const ctx = canvas.getContext("2d");
      if (!ctx) return "";
      const img = ctx.createImageData(icon.width, icon.height);
      // Clamp to the canvas buffer length: some platforms pad rows, so the
      // source RGBA can be longer than width*height*4 (set() throws if longer).
      const src = new Uint8ClampedArray(icon.rgba);
      img.data.set(src.subarray(0, img.data.length));
      ctx.putImageData(img, 0, 0);
      return canvas.toDataURL("image/png");
    })
    .catch(() => "")
    .then((url) => {
      if (!url) iconCache.delete(appKey);
      return url;
    });
  iconCache.set(appKey, p);
  return p;
}

export function AppAvatar({
  name,
  appKey,
  size = 24,
}: {
  name: string;
  appKey: string;
  size?: number;
}) {
  const [src, setSrc] = useState<string>("");

  useEffect(() => {
    let alive = true;
    const retryDelaysMs = [0, 1_500, 4_000];
    const timers: number[] = [];

    retryDelaysMs.forEach((delay, index) => {
      const timer = window.setTimeout(() => {
        loadIcon(appKey, index > 0)
          .then((url) => {
            if (alive && url) setSrc(url);
          })
          .catch(() => {});
      }, delay);
      timers.push(timer);
    });

    return () => {
      alive = false;
      timers.forEach(window.clearTimeout);
    };
  }, [appKey]);

  if (src) {
    return (
      <img
        src={src}
        alt=""
        aria-hidden
        width={size}
        height={size}
        className="inline-block shrink-0 rounded-md object-contain"
        style={{ width: size, height: size }}
      />
    );
  }

  const letter = (name.trim()[0] ?? "?").toUpperCase();
  const bg = hashColor(appKey || name);
  return (
    <span
      aria-hidden
      className="inline-flex shrink-0 items-center justify-center rounded-md font-medium text-white"
      style={{
        width: size,
        height: size,
        backgroundColor: bg,
        fontSize: Math.round(size * 0.5),
      }}
    >
      {letter}
    </span>
  );
}
