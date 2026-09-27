"use client";

import { useEffect, useState } from "react";
import { api, apiErrorMessage, DonationItem, MapPoint, ScoredMatch } from "@/lib/api";

interface GardenGraphProps {
  donation: DonationItem | null;
  matches: ScoredMatch[];
  isLoading?: boolean;
}

function isMatch(item: ScoredMatch | MapPoint): item is ScoredMatch {
  return "ngo_id" in item;
}

function uniqueTargets(items: (ScoredMatch | MapPoint)[]) {
  const seenNames = new Set<string>();
  return items.filter((item) => {
    const name = (isMatch(item) ? item.ngo_name : item.name).trim();
    if (!name || seenNames.has(name)) return false;
    seenNames.add(name);
    return true;
  });
}

function targetDetail(item: ScoredMatch | MapPoint, isDonationMode: boolean) {
  if (!isDonationMode || !isMatch(item)) return "Organización registrada";
  const distance = item.distance_km === null
    ? "distancia no disponible"
    : `${item.distance_km.toFixed(1)} km`;
  return `Prioridad: ${item.final_score.toFixed(1)}/100 | ${distance}`;
}

function centerTitle(donation: DonationItem | null) {
  if (!donation) return "Centro de Acopio";
  return donation.title.length > 20 ? `${donation.title.slice(0, 20)}...` : donation.title;
}

export function GardenGraph({ donation, matches, isLoading = false }: GardenGraphProps) {
  const [error, setError] = useState<string | null>(null);
  const [allPoints, setAllPoints] = useState<MapPoint[]>([]);
  const [selectedMatchId, setSelectedMatchId] = useState<string | null>(null);

  useEffect(() => {
    if (!donation) {
      api.getMapPoints().then(setAllPoints).catch((err) => setError(apiErrorMessage(err)));
    }
  }, [donation]);

  const activeMatch = donation ? matches.find((m) => m.ngo_id === selectedMatchId) ?? matches[0] ?? null : null;

  const isDonationMode = Boolean(donation);
  const rawList = isDonationMode ? matches : allPoints.filter((p) => p.point_type === "ong");

  // Deduplicación estricta de nombres para que nunca se encimen
  const targetList = uniqueTargets(rawList);

  const totalTargets = targetList.length;

  const width = 940;
  const height = Math.max(460, totalTargets * 74);
  const originX = 110;
  const originY = height / 2;
  const targetX = 490;

  const getTargetY = (index: number) => {
    if (totalTargets <= 1) return height / 2;
    const padding = 55;
    const step = (height - padding * 2) / (totalTargets - 1);
    return padding + index * step;
  };

  return (
    <div className="relative border border-garden-border bg-garden-surface/80  rounded-2xl p-5 overflow-hidden shadow-garden-glow">
      <div className="flex justify-between items-center mb-3">
        <div>
          <h3 className="text-sm font-semibold text-garden-text flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-garden-leaf " />
            {isDonationMode ? "Organizaciones compatibles" : "Conexiones de la comunidad"}
          </h3>
          <p className="text-sm text-garden-sage">
            {isDonationMode
              ? `Coincidencias para: ${donation?.title}`
              : "Visualización de las organizaciones y almacenes conectados en la zona"}
          </p>
        </div>

        {donation && (
          <span className="text-xs font-sans px-2.5 py-0.5 rounded-full bg-garden-dark border border-garden-border text-garden-sprout">
            {donation.quantity} unidades disponibles
          </span>
        )}
      </div>

      {error && <p role="alert" className="notice notice-error mb-3">{error}</p>}
      {!isDonationMode && (
        <div className="mb-3 px-3.5 py-2 rounded-xl bg-garden-dark/80 border border-garden-border flex items-center justify-between text-xs">
          <span className="text-garden-sage">
            💡 Haz clic en <strong className="text-garden-sprout font-medium">&quot;Ver coincidencias&quot;</strong> en tu inventario para consultar las organizaciones compatibles.
          </span>
          <span className="text-xs font-sans text-garden-sage bg-garden-surface px-2 py-0.5 rounded border border-garden-border">
            Selecciona una donación
          </span>
        </div>
      )}

      {isLoading ? (
        <div className="h-80 flex flex-col items-center justify-center border border-garden-border/40 rounded-xl bg-garden-dark/30">
          <div className="w-10 h-10 border-2 border-garden-emerald/30 border-t-garden-leaf rounded-full animate-spin mb-3" />
          <p className="text-xs text-garden-sprout font-sans">Puntuando afinidad de las organizaciones...</p>
        </div>
      ) : totalTargets === 0 ? (
        <div className="h-80 flex flex-col items-center justify-center text-center p-6 border border-dashed border-garden-border/60 rounded-xl bg-garden-dark/40">
          <p className="text-xs text-garden-text font-medium">No se detectaron organizaciones conectadas</p>
          <p className="text-sm text-garden-sage mt-1">
            Selecciona otra donación o vuelve más tarde para consultar nuevas organizaciones.
          </p>
        </div>
      ) : (
        <div className="graph-view">
          <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-auto min-w-[640px] select-none">
            <defs>
              <filter id="glow-emerald" x="-30%" y="-30%" width="160%" height="160%">
                <feGaussianBlur stdDeviation="4" result="blur" />
                <feComposite in="SourceGraphic" in2="blur" operator="over" />
              </filter>

              <linearGradient id="branch-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#059669" stopOpacity="0.8" />
                <stop offset="70%" stopColor="var(--primary)" stopOpacity="0.6" />
                <stop offset="100%" stopColor="var(--primary-hover)" stopOpacity="0.9" />
              </linearGradient>
            </defs>

            {/* RAMAS SVG */}
            {targetList.map((item, i) => {
              const destY = getTargetY(i);
              const score = isMatch(item) ? item.final_score : 70;
              const branchThickness = Math.max(1.8, (score / 100) * 5.5);
              const isSelected = isMatch(item) && activeMatch?.ngo_id === item.ngo_id;

              const cp1X = originX + (targetX - originX) * 0.45;
              const cp1Y = originY;
              const cp2X = originX + (targetX - originX) * 0.55;
              const cp2Y = destY;

              const pathData = `M ${originX} ${originY} C ${cp1X} ${cp1Y}, ${cp2X} ${cp2Y}, ${targetX} ${destY}`;

              return (
                <g key={isMatch(item) ? item.ngo_id : item.id} className="cursor-pointer" onClick={() => { if (isMatch(item)) setSelectedMatchId(item.ngo_id); }}>
                  <path d={pathData} fill="none" stroke="transparent" strokeWidth={24} />
                  <path
                    d={pathData}
                    fill="none"
                    stroke={isSelected ? "var(--primary-hover)" : "url(#branch-gradient)"}
                    strokeWidth={isSelected ? branchThickness + 2.5 : branchThickness}
                    strokeOpacity={isSelected ? 1 : 0.65}
                    strokeLinecap="round"
                    className="transition-all duration-300 hover:stroke-garden-sprout"
                  />
                  <path
                    d={pathData}
                    fill="none"
                    stroke="var(--text)"
                    strokeWidth={Math.max(1, branchThickness * 0.3)}
                    strokeDasharray="4 14"
                    strokeDashoffset={i * 8}
                    strokeOpacity={0.35}

                  />
                </g>
              );
            })}

            {/* NODO CENTRAL */}
            <g transform={`translate(${originX}, ${originY})`}>
              <circle r={26} fill="var(--surface)" stroke="var(--emerald)" strokeWidth={2}  />
              <circle r={14} fill="var(--emerald)" opacity={0.2}  />
              <circle r={7} fill="var(--primary)" />
              <text x={-40} y={-34} fill="var(--text)" fontSize="14" fontWeight="600">
                {centerTitle(donation)}
              </text>
              <text x={-40} y={-20} fill="var(--muted)" fontSize="12" className="font-sans">
                {isDonationMode ? "Lote a Distribuir" : "Referencia de la red"}
              </text>
            </g>

            {/* NODOS DESTINO */}
            {targetList.map((item, i) => {
              const destY = getTargetY(i);
              const name = isMatch(item) ? item.ngo_name : item.name;
              const isSelected = isMatch(item) && activeMatch?.ngo_id === item.ngo_id;
              const isHighPrio = isMatch(item) ? item.final_score >= 80 : true;

              return (
                <g
                  key={isMatch(item) ? item.ngo_id : item.id}
                  transform={`translate(${targetX}, ${destY})`}
                  className="cursor-pointer group"
                  role={isMatch(item) ? "button" : undefined}
                  tabIndex={isMatch(item) ? 0 : undefined}
                  aria-label={isMatch(item) ? `Ver compatibilidad con ${name}` : undefined}
                  onKeyDown={(event) => { if (isMatch(item) && (event.key === "Enter" || event.key === " ")) { event.preventDefault(); setSelectedMatchId(item.ngo_id); } }}
                  onClick={() => { if (isMatch(item)) setSelectedMatchId(item.ngo_id); }}
                >
                  <circle
                    r={isSelected ? 18 : 13}
                    fill="var(--surface)"
                    stroke={isHighPrio ? "var(--primary)" : "var(--emerald)"}
                    strokeWidth={isSelected ? 2.5 : 1.5}

                    className="transition-all duration-300 group-hover:stroke-garden-sprout"
                  />
                  <circle r={isSelected ? 6 : 4} fill={isHighPrio ? "var(--primary-hover)" : "var(--primary)"} />
                  <text
                    x={24}
                    y={-2}
                    fill={isSelected ? "var(--text)" : "var(--text)"}
                    fontSize="14"
                    fontWeight={isSelected ? "600" : "500"}
                    className="transition-colors group-hover:fill-green-800 font-sans"
                  >
                    {name}
                  </text>
                  <text x={24} y={13} fill="var(--muted)" fontSize="12" className="font-sans">
                    {targetDetail(item, isDonationMode)}
                  </text>
                </g>
              );
            })}
          </svg>
        </div>
      )}

      {/* DETALLE DEL NODO EXPANDIDO */}
      {activeMatch && (
        <div className="mt-4 p-4 rounded-xl bg-garden-dark/95 border border-garden-border flex flex-col md:flex-row justify-between items-start md:items-center gap-4 transition-all">
          <div className="flex items-start gap-3">
            <div className="w-10 h-10 rounded-xl bg-garden-surface border border-garden-emerald/40 flex items-center justify-center font-sans font-bold text-sm text-garden-sprout shrink-0">
              {activeMatch.final_score.toFixed(0)}
            </div>
            <div>
              <p className="text-xs font-semibold text-garden-text flex items-center gap-2">
                <span>{activeMatch.ngo_name}</span>
                {activeMatch.ai_priority && (
                  <span className="text-[9px] font-sans uppercase px-2 py-0.5 rounded-full bg-garden-emerald/20 text-garden-sprout border border-garden-emerald/30">
                    Prioridad {activeMatch.ai_priority}
                  </span>
                )}
              </p>
              <p className="text-sm text-garden-sage mt-0.5">
                Puntaje de contenido: {(activeMatch.semantic_similarity * 100).toFixed(1)}/100 | Distancia: {activeMatch.distance_km === null ? "no disponible" : `${activeMatch.distance_km.toFixed(1)} km`}
              </p>
              {activeMatch.ai_reasoning && (
                <p className="text-sm text-garden-leaf italic mt-1 bg-garden-surface/60 px-2.5 py-1 rounded border border-garden-border/40">
                  &ldquo;{activeMatch.ai_reasoning}&rdquo; — Explicación complementaria
                </p>
              )}
            </div>
          </div>
          <span className="text-xs font-sans px-2.5 py-1 rounded-full bg-garden-dark border border-garden-emerald/30 text-garden-leaf shrink-0">
            Compatibilidad calculada
          </span>
        </div>
      )}
    </div>
  );
}
