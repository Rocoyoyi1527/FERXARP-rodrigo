import { isDemoNgo, ngoDescription } from "@/lib/demo";
import type { MapPoint } from "@/lib/api";
export function isDemoWarehouse(point: MapPoint): boolean {
  return point.point_type === "acopio" && point.id === "00000000-0000-0000-0000-000000000000";
}
export function createMapPopupContent(point: MapPoint): HTMLElement {
  const container = document.createElement("div");
  const name = document.createElement("strong");
  name.className = "map-popup-name";
  name.textContent = isDemoWarehouse(point) ? "Almacén demo" : point.name;
  const badge = document.createElement("span");
  badge.className = "badge";
  badge.textContent = point.point_type === "acopio" ? "Punto de acopio" : (isDemoNgo(point.id) ? "Demo · Organización" : "Organización");
  const details = document.createElement("p");
  details.className = "map-popup-details";
  details.textContent = isDemoWarehouse(point) ? "Punto de referencia de la demo en Veracruz. No representa tu ubicación." : ngoDescription(point.id, point.details) ?? "";
  container.append(name, document.createElement("br"), badge, details);
  return container;
}
