"use client";

import { FormEvent, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useRouter } from "next/navigation";
import { api, apiErrorMessage, isApiError, ScanAction } from "@/lib/api";

interface StockScannerProps {
  donationId: string;
  actions: readonly ScanAction[];
  onStatusChanged: () => Promise<void> | void;
  onFeedback: (message: string, error: boolean) => void;
}

export function StockScanner({ donationId, actions, onStatusChanged, onFeedback }: StockScannerProps) {
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  const [rejectionOpen, setRejectionOpen] = useState(false);
  const [reason, setReason] = useState("");
  const [reasonError, setReasonError] = useState<string | null>(null);
  const reasonRef = useRef<HTMLTextAreaElement>(null);
  const inFlight = useRef(false);

  useEffect(() => {
    if (!rejectionOpen) return;
    reasonRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !inFlight.current) setRejectionOpen(false);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [rejectionOpen]);

  const runAction = async (action: ScanAction, rejectionReason?: string) => {
    if (inFlight.current) return;
    inFlight.current = true;
    setLoading(true);
    onFeedback("", false);
    try {
      const result = await api.shipmentAction(
        action === "rechazo"
          ? { donation_id: donationId, action, rejection_reason: rejectionReason ?? "" }
          : { donation_id: donationId, action }
      );
      setRejectionOpen(false);
      setReason("");
      onFeedback(result.message, false);
      await onStatusChanged();
    } catch (error) {
      onFeedback(apiErrorMessage(error), true);
      if (action === "rechazo" && isApiError(error, 400)) setReasonError(apiErrorMessage(error));
      if (isApiError(error, 409)) await onStatusChanged();
      if (isApiError(error, 401)) {
        localStorage.removeItem("fexarp_token");
        router.push("/login");
      }
    } finally {
      inFlight.current = false;
      setLoading(false);
    }
  };

  const submitRejection = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const value = reason.trim();
    if (!value || value.length > 500) {
      setReasonError("Escribe un motivo de entre 1 y 500 caracteres.");
      return;
    }
    setReasonError(null);
    void runAction("rechazo", value);
  };

  if (actions.length === 0) return null;

  return (
    <div className="mt-3 p-3.5 rounded-xl bg-garden-dark/95 border border-garden-border space-y-3">
      <div className="flex justify-between items-center">
        <span className="text-[10px] font-mono uppercase tracking-wider text-garden-sage">Trazabilidad física</span>
        <span className="text-[10px] font-mono text-garden-leaf bg-garden-surface px-2 py-0.5 rounded border border-garden-border">
          ID: {donationId.slice(0, 8)}...
        </span>
      </div>
      <div className="flex flex-wrap gap-2">
        {actions.includes("salida") && (
          <button type="button" disabled={loading} onClick={() => void runAction("salida")}
            className="flex-1 text-xs py-2 px-3 rounded-lg bg-garden-emerald text-garden-obsidian font-semibold disabled:opacity-50">
            {loading ? "Registrando..." : "Registrar salida"}
          </button>
        )}
        {actions.includes("entrega") && (
          <button type="button" disabled={loading} onClick={() => void runAction("entrega")}
            className="flex-1 text-xs py-2 px-3 rounded-lg bg-garden-emerald text-garden-obsidian font-semibold disabled:opacity-50">
            {loading ? "Registrando..." : "Marcar como entregada"}
          </button>
        )}
        {actions.includes("rechazo") && (
          <button type="button" disabled={loading} onClick={() => setRejectionOpen(true)}
            className="flex-1 text-xs py-2 px-3 rounded-lg border border-rose-700 text-rose-200 disabled:opacity-50">
            Rechazar entrega
          </button>
        )}
      </div>

      {rejectionOpen && createPortal(
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4">
          <div role="dialog" aria-modal="true" aria-labelledby="rejection-title"
            className="w-full max-w-md rounded-2xl border border-garden-border bg-garden-surface p-6 shadow-garden-glow">
            <h3 id="rejection-title" className="text-base font-semibold text-white">Registrar rechazo</h3>
            <p className="mt-1 text-xs text-garden-sage">El motivo quedará registrado en la entrega.</p>
            <form onSubmit={submitRejection} className="mt-4 space-y-4">
              <div>
                <label htmlFor="rejection-reason" className="block text-xs text-garden-sage mb-1">Motivo del rechazo</label>
                <textarea id="rejection-reason" ref={reasonRef} value={reason} maxLength={500} rows={4}
                  onChange={(event) => { setReason(event.target.value); setReasonError(null); }}
                  aria-invalid={Boolean(reasonError)} aria-describedby={reasonError ? "rejection-error" : undefined}
                  className="w-full rounded-xl border border-garden-border bg-garden-dark p-3 text-sm text-white focus:outline-none focus:border-garden-leaf" />
                {reasonError && <p id="rejection-error" className="mt-1 text-xs text-rose-300">{reasonError}</p>}
              </div>
              <div className="flex justify-end gap-2">
                <button type="button" disabled={loading} onClick={() => { setRejectionOpen(false); setReason(""); setReasonError(null); }}
                  className="rounded-xl border border-garden-border px-4 py-2 text-xs text-garden-sage disabled:opacity-50">Cancelar</button>
                <button type="submit" disabled={loading || !reason.trim()}
                  className="rounded-xl bg-rose-700 px-4 py-2 text-xs font-semibold text-white disabled:opacity-50">
                  {loading ? "Registrando..." : "Confirmar rechazo"}
                </button>
              </div>
            </form>
          </div>
        </div>,
        document.body
      )}
    </div>
  );
}
