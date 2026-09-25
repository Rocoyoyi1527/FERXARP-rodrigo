"use client";

import { useEffect, useState } from "react";
import { api, DonationItem, MapPoint, ScoredMatch } from "@/lib/api";

interface GardenGraphProps {
  donation: DonationItem | null;
  matches: ScoredMatch[];
  isLoading?: boolean;
}

function isMatch(item: ScoredMatch | MapPoint): item is ScoredMatch {
  return "ngo_id" in item;
}

export function GardenGraph({ donation, matches, isLoading = false }: GardenGraphProps) {
  const [allPoints, setAllPoints] = useState<MapPoint[]>([]);
  const [selectedMatchId, setSelectedMatchId] = useState<string | null>(null);

  useEffect(() => {
    if (!donation) {
      api.getMapPoints().then(setAllPoints).catch(console.error);
    }
  }, [donation]);

  const activeMatch = donation ? matches.find((m) => m.ngo_id === selectedMatchId) ?? matches[0] ?? null : null;

  const isDonationMode = Boolean(donation);
  const rawList = isDonationMode ? matches : allPoints.filter((p) => p.point_type === "ong");

  // Deduplicación estricta de nombres para que nunca se encimen
  const seenNames = new Set<string>();
  const targetList = rawList.filter((item) => {
    const name = (isMatch(item) ? item.ngo_name : item.name).trim();
    if (!name || seenNames.has(name)) return false;
    seenNames.add(name);
    return true;
  });

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
    <div className="relative border border-garden-border bg-garden-surface/80 backdrop-blur-md rounded-2xl p-5 overflow-hidden shadow-garden-glow">
      <div className="flex justify-between items-center mb-3">
        <div>
          <h3 className="text-sm font-semibold text-white flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-garden-leaf animate-pulse" />
            {isDonationMode ? "Ramas de Asignación Inteligente" : "Red General de Organizaciones y Empresas"}
          </h3>
          <p className="text-[11px] text-garden-sage">
            {isDonationMode
              ? `Calculando afinidad con IA para: ${donation?.title}`
              : "Visualización de las organizaciones y almacenes conectados en la zona"}
          </p>
        </div>

        {donation && (
          <span className="text-[10px] font-mono px-2.5 py-0.5 rounded-full bg-garden-dark border border-garden-border text-garden-sprout">
            {donation.quantity} unidades disponibles
          </span>
        )}
      </div>

      {!isDonationMode && (
        <div className="mb-3 px-3.5 py-2 rounded-xl bg-garden-dark/80 border border-garden-border flex items-center justify-between text-xs">
          <span className="text-garden-sage">
            💡 Haz clic en <strong className="text-garden-sprout font-medium">&quot;Ramificar en Árbol 🌿&quot;</strong> en tu inventario para activar la evaluación de <strong className="text-white">DeepSeek-R1 (Groq)</strong>.
          </span>
          <span className="text-[10px] font-mono text-neutral-400 bg-garden-surface px-2 py-0.5 rounded border border-garden-border">
            IA en espera
          </span>
        </div>
      )}

      {isLoading ? (
        <div className="h-80 flex flex-col items-center justify-center border border-garden-border/40 rounded-xl bg-garden-dark/30">
          <div className="w-10 h-10 border-2 border-garden-emerald/30 border-t-garden-leaf rounded-full animate-spin mb-3" />
          <p className="text-xs text-garden-sprout font-mono">Puntuando afinidad y vectores con DeepSeek-R1...</p>
        </div>
      ) : totalTargets === 0 ? (
        <div className="h-80 flex flex-col items-center justify-center text-center p-6 border border-dashed border-garden-border/60 rounded-xl bg-garden-dark/40">
          <p className="text-xs text-neutral-300 font-medium">No se detectaron organizaciones conectadas</p>
          <p className="text-[11px] text-garden-sage mt-1">
            Pulsa el botón superior &quot;Poblar Localidad&quot; para cargar entidades reales de la región.
          </p>
        </div>
      ) : (
        <div className="relative w-full overflow-x-auto">
          <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-auto min-w-[780px] select-none">
            <defs>
              <filter id="glow-emerald" x="-30%" y="-30%" width="160%" height="160%">
                <feGaussianBlur stdDeviation="4" result="blur" />
                <feComposite in="SourceGraphic" in2="blur" operator="over" />
              </filter>

              <linearGradient id="branch-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#059669" stopOpacity="0.8" />
                <stop offset="70%" stopColor="#34d399" stopOpacity="0.6" />
                <stop offset="100%" stopColor="#6ee7b7" stopOpacity="0.9" />
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
                    stroke={isSelected ? "#6ee7b7" : "url(#branch-gradient)"}
                    strokeWidth={isSelected ? branchThickness + 2.5 : branchThickness}
                    strokeOpacity={isSelected ? 1 : 0.65}
                    strokeLinecap="round"
                    className="transition-all duration-300 hover:stroke-garden-sprout"
                  />
                  <path
                    d={pathData}
                    fill="none"
                    stroke="#ffffff"
                    strokeWidth={Math.max(1, branchThickness * 0.3)}
                    strokeDasharray="4 14"
                    strokeDashoffset={i * 8}
                    strokeOpacity={0.35}
                    className="animate-[dash_12s_linear_infinite]"
                  />
                </g>
              );
            })}

            {/* NODO CENTRAL */}
            <g transform={`translate(${originX}, ${originY})`}>
              <circle r={26} fill="#0f1913" stroke="#10b981" strokeWidth={2} filter="url(#glow-emerald)" />
              <circle r={14} fill="#10b981" opacity={0.2} className="animate-ping" />
              <circle r={7} fill="#34d399" />
              <text x={-40} y={-34} fill="#ffffff" fontSize="11" fontWeight="600">
                {isDonationMode ? (donation!.title.length > 20 ? `${donation!.title.slice(0, 20)}...` : donation!.title) : "Centro de Acopio"}
              </text>
              <text x={-40} y={-20} fill="#8fa896" fontSize="9" className="font-mono">
                {isDonationMode ? "Lote a Distribuir" : "Hub Logístico"}
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
                  onClick={() => { if (isMatch(item)) setSelectedMatchId(item.ngo_id); }}
                >
                  <circle
                    r={isSelected ? 18 : 13}
                    fill="#0a110d"
                    stroke={isHighPrio ? "#34d399" : "#10b981"}
                    strokeWidth={isSelected ? 2.5 : 1.5}
                    filter={isHighPrio ? "url(#glow-emerald)" : undefined}
                    className="transition-all duration-300 group-hover:stroke-garden-sprout"
                  />
                  <circle r={isSelected ? 6 : 4} fill={isHighPrio ? "#6ee7b7" : "#34d399"} />
                  <text
                    x={24}
                    y={-2}
                    fill={isSelected ? "#ffffff" : "#d1d5db"}
                    fontSize="11"
                    fontWeight={isSelected ? "600" : "500"}
                    className="transition-colors group-hover:fill-white font-sans"
                  >
                    {name}
                  </text>
                  <text x={24} y={13} fill="#8fa896" fontSize="9.5" className="font-mono">
                    {isDonationMode
                      ? isMatch(item) ? `Prioridad: ${item.final_score.toFixed(1)}/100 | ${item.distance_km === null ? "distancia no disponible" : `${item.distance_km.toFixed(1)} km`}` : "Organización Verificada"
                      : "Organización Verificada"}
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
            <div className="w-10 h-10 rounded-xl bg-garden-surface border border-garden-emerald/40 flex items-center justify-center font-mono font-bold text-sm text-garden-sprout shrink-0">
              {activeMatch.final_score.toFixed(0)}
            </div>
            <div>
              <p className="text-xs font-semibold text-white flex items-center gap-2">
                <span>{activeMatch.ngo_name}</span>
                {activeMatch.ai_priority && (
                  <span className="text-[9px] font-mono uppercase px-2 py-0.5 rounded-full bg-garden-emerald/20 text-garden-sprout border border-garden-emerald/30">
                    Prioridad {activeMatch.ai_priority}
                  </span>
                )}
              </p>
              <p className="text-[11px] text-garden-sage mt-0.5">
                Puntaje de contenido: {(activeMatch.semantic_similarity * 100).toFixed(1)}/100 | Distancia: {activeMatch.distance_km === null ? "no disponible" : `${activeMatch.distance_km.toFixed(1)} km`}
              </p>
              {activeMatch.ai_reasoning && (
                <p className="text-[11px] text-emerald-300/90 italic mt-1 bg-garden-surface/60 px-2.5 py-1 rounded border border-garden-border/40">
                  &ldquo;{activeMatch.ai_reasoning}&rdquo; — DeepSeek R1 (Groq)
                </p>
              )}
            </div>
          </div>
          <span className="text-[10px] font-mono px-2.5 py-1 rounded-full bg-garden-dark border border-garden-emerald/30 text-garden-leaf shrink-0">
            Afinidad Vectorial Validada
          </span>
        </div>
      )}
    </div>
  );
}
