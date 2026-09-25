"use client";

import { useState } from "react";
import Link from "next/link";
import { DonationItem, DonationState } from "@/lib/api";

interface DonationListProps {
  donations: DonationItem[];
  onSelectMatching: (id: string) => void;
}

export function DonationList({
  donations,
  onSelectMatching,
}: DonationListProps) {
  const [activeDonationId, setActiveDonationId] = useState<string | null>(null);

  const getStatusBadge = (status: DonationState) => {
    switch (status) {
      case "reservado":
        return <span className="text-[10px] font-mono text-amber-300">Reservado</span>;
      case "en_transito":
        return (
          <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-amber-950/40 border border-amber-600/50 text-amber-300 flex items-center gap-1">
            <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse" />
            Tallo en Ruta
          </span>
        );
      case "entregado":
        return (
          <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-emerald-950/40 border border-emerald-500/50 text-garden-sprout flex items-center gap-1">
            <span className="w-1.5 h-1.5 rounded-full bg-garden-leaf" />
            Fruto Cosechado
          </span>
        );
      case "rechazado":
        return <span className="text-[10px] font-mono text-rose-300">Rechazado</span>;
      case "en_acopio":
        return (
          <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-garden-surface border border-garden-border text-garden-sage flex items-center gap-1">
            <span className="w-1.5 h-1.5 rounded-full bg-garden-leaf/60" />
            Semilla en Acopio
          </span>
        );
    }
  };

  return (
    <div className="border border-garden-border bg-garden-surface/80 backdrop-blur-md rounded-2xl p-5 shadow-garden-glow">
      <div className="flex justify-between items-center mb-4">
        <div>
          <h2 className="text-sm font-semibold text-white tracking-tight">Inventario de Semillas</h2>
          <p className="text-[11px] text-garden-sage">Lotes activos listos para ramificación o escaneo</p>
        </div>
        <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-garden-dark border border-garden-border text-garden-leaf">
          {donations.length} {donations.length === 1 ? "Lote" : "Lotes"}
        </span>
      </div>

      {donations.length === 0 ? (
        <div className="text-center py-8 border border-dashed border-garden-border/60 rounded-xl bg-garden-dark/30">
          <p className="text-xs text-garden-sage">No hay lotes sembrados en tu perfil.</p>
          <p className="text-[11px] text-neutral-500 mt-0.5">Usa el formulario superior para registrar uno.</p>
        </div>
      ) : (
        <div className="space-y-3">
          {donations.map((d) => {
            const isSelected = activeDonationId === d.id;

            return (
              <div
                key={d.id}
                className={`border rounded-xl p-3.5 transition-all duration-200 ${
                  isSelected
                    ? "border-garden-leaf bg-garden-dark/90 shadow-[0_0_15px_rgba(52,211,153,0.1)]"
                    : "border-garden-border bg-garden-dark/50 hover:border-garden-border-glow"
                }`}
              >
                <div className="flex justify-between items-start gap-2">
                  <div>
                    <h3 className="text-xs font-semibold text-white leading-snug">{d.title}</h3>
                    {d.description && (
                      <p className="text-[11px] text-garden-sage line-clamp-1 mt-0.5">{d.description}</p>
                    )}
                    <p className="text-[11px] font-mono text-neutral-400 mt-1">
                      Volumen: <span className="text-garden-sprout font-bold">{d.quantity}</span> unidades
                    </p>
                  </div>
                  <div>{getStatusBadge(d.status)}</div>
                </div>

                {/* Acciones del Lote */}
                <div className="flex items-center gap-2 mt-3 pt-2.5 border-t border-garden-border/40">
                  <button
                    type="button"
                    onClick={() => {
                      setActiveDonationId(d.id);
                      onSelectMatching(d.id);
                    }}
                    className={`flex-1 text-[11px] py-1.5 px-2.5 rounded-lg font-medium transition cursor-pointer flex items-center justify-center gap-1.5 ${
                      isSelected
                        ? "bg-garden-leaf text-garden-obsidian font-semibold shadow-[0_0_10px_#34d399]"
                        : "bg-garden-surface hover:bg-garden-card text-neutral-200 border border-garden-border"
                    }`}
                  >
                    <span>Ramificar en Árbol</span>
                    <span>🌿</span>
                  </button>

                  <Link
                    href="/shipments"
                    className="text-[11px] py-1.5 px-3 rounded-lg bg-garden-dark hover:bg-garden-surface text-garden-sage hover:text-white border border-garden-border transition cursor-pointer"
                  >
                    Ver envíos
                  </Link>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
