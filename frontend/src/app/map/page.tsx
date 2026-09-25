"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { api, MapPoint } from "@/lib/api";
import type { Layer, Map as LeafletMap, Marker } from "leaflet";

export default function MapPage() {
  const router = useRouter();
  const mapContainerRef = useRef<HTMLDivElement>(null);
  const mapInstanceRef = useRef<LeafletMap | null>(null);
  const [points, setPoints] = useState<MapPoint[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const token = localStorage.getItem("fexarp_token");
    if (!token) {
      router.push("/login");
      return;
    }

    api.getMapPoints()
      .then((data) => {
        setPoints(data);
        setLoading(false);
      })
      .catch((err) => {
        console.error("Error al obtener puntos:", err);
        setLoading(false);
      });
  }, [router]);

  useEffect(() => {
    if (loading || !mapContainerRef.current) return;

    let isMounted = true;

    import("leaflet").then((L) => {
      if (!isMounted || !mapContainerRef.current) return;

      if (!document.getElementById("leaflet-css")) {
        const link = document.createElement("link");
        link.id = "leaflet-css";
        link.rel = "stylesheet";
        link.href = "https://unpkg.com/leaflet@1.9.4/dist/leaflet.css";
        document.head.appendChild(link);
      }

      if (!document.getElementById("map-botanical-filter")) {
        const style = document.createElement("style");
        style.id = "map-botanical-filter";
        style.innerHTML = `
          .botanical-tiles .leaflet-tile {
            filter: brightness(0.65) invert(1) contrast(1.7) hue-rotate(185deg) saturate(0.35) !important;
          }
          .custom-popup .leaflet-popup-content-wrapper {
            background: #0f1913;
            border: 1px solid rgba(36, 62, 49, 0.8);
            border-radius: 14px;
            color: #e5e7eb;
            box-shadow: 0 0 20px rgba(16, 185, 129, 0.2);
          }
          .custom-popup .leaflet-popup-tip {
            background: #0f1913;
            border: 1px solid rgba(36, 62, 49, 0.8);
          }
          .leaflet-control-attribution {
            display: none !important;
          }
        `;
        document.head.appendChild(style);
      }

      // Limpieza segura previa
      if (mapInstanceRef.current) {
        try {
          mapInstanceRef.current.remove();
        } catch {
          // Ignorar si el DOM ya se recicló
        }
        mapInstanceRef.current = null;
      }
      const mapContainer = mapContainerRef.current as HTMLDivElement & { _leaflet_id?: number };
      if (mapContainer._leaflet_id) {
        delete mapContainer._leaflet_id;
      }

      const map = L.map(mapContainerRef.current, {
        zoomControl: false,
        attributionControl: false,
        fadeAnimation: false,
        markerZoomAnimation: false,
      }).setView([19.1738, -96.1342], 12);
      mapInstanceRef.current = map;

      L.control.zoom({ position: "bottomright" }).addTo(map);

      L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
        className: "botanical-tiles",
        maxZoom: 19,
      }).addTo(map);

      const acopioPoint = points.find((p) => p.point_type === "acopio");
      const markersGroup: Marker[] = [];

      points.forEach((p) => {
        const isAcopio = p.point_type === "acopio";

        const iconHtml = isAcopio
          ? `<div class="relative flex items-center justify-center w-8 h-8">
               <div class="absolute w-8 h-8 rounded-full bg-emerald-500/20 animate-ping"></div>
               <div class="w-6 h-6 rounded-full bg-[#0a110d] border-2 border-emerald-400 flex items-center justify-center shadow-[0_0_12px_#34d399]">
                 <div class="w-2 h-2 rounded-full bg-emerald-300"></div>
               </div>
             </div>`
          : `<div class="relative flex items-center justify-center w-6 h-6">
               <div class="w-5 h-5 rounded-full bg-[#0a110d] border-2 border-emerald-500 flex items-center justify-center shadow-[0_0_8px_rgba(16,185,129,0.5)]">
                 <div class="w-1.5 h-1.5 rounded-full bg-[#6ee7b7]"></div>
               </div>
             </div>`;

        const customIcon = L.divIcon({
          className: "bg-transparent border-0",
          html: iconHtml,
          iconSize: isAcopio ? [32, 32] : [24, 24],
          iconAnchor: isAcopio ? [16, 16] : [12, 12],
        });

        const marker = L.marker([p.latitude, p.longitude], { icon: customIcon }).addTo(map);
        markersGroup.push(marker);

        marker.bindPopup(
          `
          <div style="font-family: inherit; padding: 4px;">
            <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 4px;">
              <strong style="color: #ffffff; font-size: 13px;">${p.name}</strong>
              <span style="font-size: 9px; font-family: monospace; text-transform: uppercase; padding: 2px 6px; border-radius: 9999px; background: #060907; border: 1px solid rgba(36, 62, 49, 0.8); color: ${isAcopio ? "#34d399" : "#6ee7b7"};">
                ${isAcopio ? "Almacén Central" : "Organización"}
              </span>
            </div>
            <p style="margin: 0; color: #8fa896; font-size: 11px; line-height: 1.4;">${p.details}</p>
          </div>
          `,
          { className: "custom-popup" }
        );

        if (!isAcopio && acopioPoint) {
          L.polyline(
            [
              [acopioPoint.latitude, acopioPoint.longitude],
              [p.latitude, p.longitude],
            ],
            {
              color: "#34d399",
              weight: 2,
              opacity: 0.5,
              dashArray: "4, 8",
            }
          ).addTo(map);
        }
      });

      // Encuadre sin animación para evitar race condition en el DOM
      if (markersGroup.length > 0 && isMounted) {
        const group = L.featureGroup(markersGroup);
        map.fitBounds(group.getBounds().pad(0.12), { animate: false });
      }

      setTimeout(() => {
        if (isMounted && mapInstanceRef.current) {
          mapInstanceRef.current.invalidateSize({ animate: false });
        }
      }, 100);
    });

    return () => {
      isMounted = false;
      if (mapInstanceRef.current) {
        try {
          const map = mapInstanceRef.current;
          map.eachLayer((layer: Layer) => {
            map.removeLayer(layer);
          });
          map.remove();
        } catch {
          // Captura silenciosa de desmonte
        }
        mapInstanceRef.current = null;
      }
    };
  }, [loading, points]);

  return (
    <main className="min-h-screen bg-garden-obsidian text-neutral-100 p-6 flex flex-col justify-between">
      <header className="max-w-7xl w-full mx-auto flex justify-between items-center border border-garden-border bg-garden-surface/70 backdrop-blur-md px-6 py-3.5 rounded-2xl mb-4 shadow-garden-glow">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-garden-dark border border-garden-emerald/30 flex items-center justify-center shadow-inner">
            <span className="text-sm">🌱</span>
          </div>
          <div>
            <h1 className="text-base font-semibold tracking-tight text-white">Mapa de Almacenes y Envíos</h1>
            <p className="text-xs text-garden-sage">Ubicación de centros de acopio y organizaciones en la localidad</p>
          </div>
        </div>

        <Link
          href="/dashboard"
          className="text-xs bg-garden-dark hover:bg-garden-surface text-neutral-200 border border-garden-border hover:border-garden-emerald/40 px-3.5 py-1.5 rounded-xl transition flex items-center gap-1.5 cursor-pointer"
        >
          <span>←</span>
          <span>Regresar al Tablero</span>
        </Link>
      </header>

      <div className="max-w-7xl w-full mx-auto flex-1 flex flex-col gap-3">
        <div className="flex items-center gap-6 px-1 text-xs">
          <div className="flex items-center gap-2">
            <div className="w-2.5 h-2.5 rounded-full bg-garden-emerald shadow-[0_0_8px_#10b981]" />
            <span className="text-garden-sage">Almacén Central (Hub de Acopio)</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-2.5 h-2.5 rounded-full bg-garden-leaf shadow-[0_0_6px_#34d399]" />
            <span className="text-garden-sage">Organizaciones Sociales Vinculadas</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-6 h-0 border-t-2 border-dashed border-garden-leaf/60" />
            <span className="text-garden-sage font-mono text-[11px]">Rutas Logísticas Activas</span>
          </div>
        </div>

        <div className="flex-1 w-full rounded-2xl overflow-hidden border border-garden-border relative bg-garden-surface shadow-garden-glow min-h-[580px]">
          {loading && (
            <div className="absolute inset-0 flex items-center justify-center bg-garden-obsidian/85 z-20 text-xs font-mono text-garden-sprout">
              Sincronizando coordenadas de la red...
            </div>
          )}
          <div
            ref={mapContainerRef}
            className="w-full h-full min-h-[580px]"
            style={{ width: "100%", height: "100%", minHeight: "580px" }}
          />
        </div>
      </div>
    </main>
  );
}
