"use client";

import { useState } from "react";
import Link from "next/link";
import { api, apiErrorMessage, isApiError, UserClaims } from "@/lib/api";

interface NavbarProps {
  user: UserClaims | null;
  onLogout: () => void;
}

interface AiMatch {
  donation_title: string;
  recommended_ngo: string;
  score: number;
  priority: string;
  reasoning: string;
}

interface SeedSummary {
  companies_seeded: number;
  ngos_seeded: number;
  donations_seeded: number;
  ai_evaluations: AiMatch[];
}

export function Navbar({ user, onLogout }: NavbarProps) {
  const [seeding, setSeeding] = useState(false);
  const [showConfirmModal, setShowConfirmModal] = useState(false);
  const [seedResult, setSeedResult] = useState<SeedSummary | null>(null);
  const [seedError, setSeedError] = useState<string | null>(null);

  const executeSeed = async () => {
    setShowConfirmModal(false);
    setSeeding(true);
    try {
      const res = await api.seedVeracruzData();
      setSeedResult({
        companies_seeded: res.companies_seeded,
        ngos_seeded: res.ngos_seeded,
        donations_seeded: res.donations_seeded,
        ai_evaluations: res.ai_evaluations || [],
      });
    } catch (error) {
      setSeedError(apiErrorMessage(error));
      if (isApiError(error, 401)) onLogout();
    } finally {
      setSeeding(false);
    }
  };

  const getReadableRole = (role?: string) => {
    switch (role) {
      case "empresa":
        return "Empresa Donante";
      case "ong":
        return "Organización Social";
      case "ceo":
        return "Dirección / Reportes";
      case "admin":
        return "Administrador TI";
      default:
        return "Usuario";
    }
  };

  return (
    <>
      <header className="max-w-6xl mx-auto flex flex-col sm:flex-row justify-between items-center gap-4 border border-garden-border bg-garden-surface/70 backdrop-blur-md px-6 py-4 rounded-2xl mb-8 shadow-garden-glow transition-all">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-garden-dark border border-garden-emerald/40 flex items-center justify-center shadow-inner">
            <span className="text-sm">🌱</span>
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="text-base font-semibold tracking-tight text-white">Fexarp</h1>
              <span className="text-[10px] uppercase font-mono tracking-wider px-2 py-0.5 rounded-full bg-garden-emerald/10 border border-garden-emerald/20 text-garden-leaf">
                Red Solidaria
              </span>
            </div>
            <p className="text-xs text-garden-sage">Plataforma de entrega y distribución de excedentes</p>
          </div>
        </div>

        {user && (
          <nav className="flex flex-wrap items-center gap-2.5">
            <Link
              href="/dashboard"
              className="text-xs bg-garden-dark/80 hover:bg-garden-surface text-neutral-200 hover:text-white border border-garden-border px-3.5 py-1.5 rounded-xl transition"
            >
              Tablero Principal
            </Link>

            <Link
              href="/shipments"
              className="text-xs bg-garden-dark/80 hover:bg-garden-surface text-neutral-200 hover:text-white border border-garden-border px-3.5 py-1.5 rounded-xl transition flex items-center gap-1.5"
            >
              <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
              Envíos y Solicitudes
            </Link>

            <Link
              href="/map"
              className="text-xs bg-garden-dark/80 hover:bg-garden-surface text-neutral-200 hover:text-white border border-garden-border px-3.5 py-1.5 rounded-xl transition flex items-center gap-1.5"
            >
              <span className="w-1.5 h-1.5 rounded-full bg-garden-leaf" />
              Mapa de Almacenes
            </Link>

            {user.role === "admin" && <button
              type="button"
              disabled={seeding}
              onClick={() => setShowConfirmModal(true)}
              className="text-xs bg-emerald-950/40 hover:bg-emerald-900/50 text-garden-sprout border border-emerald-500/40 px-3 py-1.5 rounded-xl transition cursor-pointer disabled:opacity-50 font-sans flex items-center gap-1.5"
            >
              <span>✨</span>
              <span>{seeding ? "Evaluando con IA..." : "Poblar Localidad"}</span>
            </button>}

            <div className="flex items-center gap-1.5 bg-garden-dark border border-garden-border px-3 py-1 rounded-full text-xs">
              <span className="text-garden-sage text-[11px]">Cuenta:</span>
              <span className="font-semibold text-garden-sprout">{getReadableRole(user.role)}</span>
            </div>

            <button
              onClick={onLogout}
              className="text-xs text-neutral-400 hover:text-rose-400 px-2 py-1 rounded transition cursor-pointer"
            >
              Cerrar Sesión
            </button>
          </nav>
        )}
      </header>

      {seedError && <p role="status" className="mx-auto mb-4 max-w-6xl rounded-xl border border-rose-800 p-3 text-xs text-rose-300">{seedError}</p>}

      {/* MODAL 1: Confirmación */}
      {showConfirmModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4">
          <div className="w-full max-w-md border border-garden-border bg-garden-surface p-6 rounded-2xl shadow-garden-glow space-y-4">
            <div className="flex items-center gap-3">
              <div className="w-9 h-9 rounded-xl bg-garden-emerald/20 border border-garden-emerald/40 flex items-center justify-center text-garden-sprout text-lg">
                ✨
              </div>
              <div>
                <h3 className="text-sm font-semibold text-white">Poblar Datos y Evaluar con IA</h3>
                <p className="text-xs text-garden-sage">DeepSeek-R1 (Groq) evaluará la afinidad de cada lote</p>
              </div>
            </div>

            <p className="text-xs text-neutral-300 leading-relaxed">
              Se sincronizarán las entidades reales de la región, se actualizarán los vectores en ChromaDB y el LLM evaluará la compatibilidad inicial de las donaciones contra las demandas sociales.
            </p>

            <div className="flex justify-end gap-2.5 pt-2">
              <button
                type="button"
                onClick={() => setShowConfirmModal(false)}
                className="text-xs px-4 py-2 rounded-xl bg-garden-dark text-neutral-300 border border-garden-border transition cursor-pointer"
              >
                Cancelar
              </button>
              <button
                type="button"
                onClick={executeSeed}
                className="text-xs px-4 py-2 rounded-xl bg-gradient-to-r from-garden-emerald to-garden-leaf text-garden-obsidian font-semibold transition cursor-pointer"
              >
                Confirmar y Puntuar
              </button>
            </div>
          </div>
        </div>
      )}

      {/* MODAL 2: Resumen con Puntuaciones de DeepSeek-R1 */}
      {seedResult && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4">
          <div className="w-full max-w-lg border border-garden-border bg-garden-surface p-6 rounded-2xl shadow-garden-glow space-y-4 max-h-[85vh] overflow-y-auto">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-garden-emerald/20 border border-garden-emerald/40 flex items-center justify-center text-garden-leaf text-xl">
                🌱
              </div>
              <div>
                <h3 className="text-sm font-semibold text-white">Ecosistema Poblado y Puntuado con IA</h3>
                <p className="text-xs text-garden-sage">Evaluaciones cognitivas generadas por DeepSeek-R1 (Groq)</p>
              </div>
            </div>

            <div className="grid grid-cols-3 gap-2 text-center text-xs">
              <div className="p-2.5 rounded-xl bg-garden-dark border border-garden-border">
                <p className="font-bold text-white">{seedResult.companies_seeded}</p>
                <p className="text-[10px] text-garden-sage">Empresas</p>
              </div>
              <div className="p-2.5 rounded-xl bg-garden-dark border border-garden-border">
                <p className="font-bold text-garden-sprout">{seedResult.ngos_seeded}</p>
                <p className="text-[10px] text-garden-sage">ONGs</p>
              </div>
              <div className="p-2.5 rounded-xl bg-garden-dark border border-garden-border">
                <p className="font-bold text-garden-leaf">{seedResult.donations_seeded}</p>
                <p className="text-[10px] text-garden-sage">Lotes Nuevos</p>
              </div>
            </div>

            {seedResult.ai_evaluations.length > 0 && (
              <div className="space-y-2.5 pt-1">
                <p className="text-[11px] font-mono text-garden-sage uppercase tracking-wider">
                  Primeras Coincidencias Validadas por el LLM:
                </p>
                {seedResult.ai_evaluations.map((ev, i) => (
                  <div key={i} className="p-3 rounded-xl bg-garden-dark border border-garden-border text-xs">
                    <div className="flex justify-between items-center">
                      <span className="font-semibold text-white">{ev.donation_title}</span>
                      <span className="font-mono text-garden-sprout font-bold">{ev.score.toFixed(0)}%</span>
                    </div>
                    <p className="text-[11px] text-garden-leaf mt-0.5">➔ {ev.recommended_ngo}</p>
                    <p className="text-[10px] text-neutral-400 italic mt-1">&ldquo;{ev.reasoning}&rdquo;</p>
                  </div>
                ))}
              </div>
            )}

            <div className="flex justify-end pt-2">
              <button
                type="button"
                onClick={() => {
                  setSeedResult(null);
                  window.location.reload();
                }}
                className="text-xs px-5 py-2.5 rounded-xl bg-gradient-to-r from-garden-emerald to-garden-leaf text-garden-obsidian font-semibold transition cursor-pointer shadow-[0_0_15px_rgba(16,185,129,0.3)]"
              >
                Aceptar y Ver en Ecosistema
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
