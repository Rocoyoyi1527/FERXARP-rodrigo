import type { MapPoint } from "@/lib/api";

export function createMapPopupContent(point: MapPoint): HTMLElement {
  const isAcopio = point.point_type === "acopio";
  const container = document.createElement("div");
  container.style.cssText = "font-family: inherit; padding: 4px";

  const heading = document.createElement("div");
  heading.style.cssText = "display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 4px";

  const name = document.createElement("strong");
  name.style.cssText = "color: #ffffff; font-size: 13px";
  name.textContent = point.name;

  const badge = document.createElement("span");
  badge.style.cssText = `font-size: 9px; font-family: monospace; text-transform: uppercase; padding: 2px 6px; border-radius: 9999px; background: #060907; border: 1px solid rgba(36, 62, 49, 0.8); color: ${isAcopio ? "#34d399" : "#6ee7b7"}`;
  badge.textContent = isAcopio ? "Almacén Central" : "Organización";

  const details = document.createElement("p");
  details.style.cssText = "margin: 0; color: #8fa896; font-size: 11px; line-height: 1.4";
  details.textContent = point.details;

  heading.append(name, badge);
  container.append(heading, details);
  return container;
}
