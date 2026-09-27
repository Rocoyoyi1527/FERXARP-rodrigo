"use client";

import { FormEvent, useRef, useState } from "react";
import { Modal } from "@/components/ui/Modal";
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
  const inFlight = useRef(false);

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
      const messages: Record<ScanAction, string> = {
        salida: "Salida registrada. La donación está en camino.",
        entrega: "Entrega registrada correctamente.",
        rechazo: "Rechazo registrado correctamente.",
      };
      onFeedback(result.message.startsWith("Lote actualizado a:") ? messages[action] : result.message, false);
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
        <span className="text-xs font-sans uppercase tracking-wider text-garden-sage">Trazabilidad física</span>
        <span className="text-xs font-sans text-garden-leaf bg-garden-surface px-2 py-0.5 rounded border border-garden-border">
          ID: {donationId.slice(0, 8)}...
        </span>
      </div>
      <div className="flex flex-wrap gap-2">
        {actions.includes("salida") && (
          <button type="button" disabled={loading} onClick={() => void runAction("salida")}
            className="btn btn-info flex-1">
            {loading ? "Registrando..." : "Registrar salida"}
          </button>
        )}
        {actions.includes("entrega") && (
          <button type="button" disabled={loading} onClick={() => void runAction("entrega")}
            className="btn btn-primary flex-1">
            {loading ? "Registrando..." : "Marcar como entregada"}
          </button>
        )}
        {actions.includes("rechazo") && (
          <button type="button" disabled={loading} onClick={() => setRejectionOpen(true)}
            className="btn btn-danger flex-1">
            Rechazar entrega
          </button>
        )}
      </div>

      {rejectionOpen && <Modal title="Registrar rechazo" busy={loading} onClose={() => setRejectionOpen(false)}>
            <p className="mt-1 text-xs text-garden-sage">El motivo quedará registrado en la entrega.</p>
            <form onSubmit={submitRejection} className="mt-4 space-y-4">
              <div>
                <label htmlFor="rejection-reason" className="block text-xs text-garden-sage mb-1">Motivo del rechazo</label>
                <textarea id="rejection-reason" value={reason} maxLength={500} rows={4}
                  onChange={(event) => { setReason(event.target.value); setReasonError(null); }}
                  aria-invalid={Boolean(reasonError)} aria-describedby={reasonError ? "rejection-error" : undefined}
                  className="field" />
                {reasonError && <p id="rejection-error" className="mt-1 text-xs text-red-700">{reasonError}</p>}
              </div>
              <div className="flex justify-end gap-2">
                <button type="button" disabled={loading} onClick={() => { setRejectionOpen(false); setReason(""); setReasonError(null); }}
                  className="btn btn-secondary">Cancelar</button>
                <button type="submit" disabled={loading || !reason.trim()}
                  className="btn btn-danger-solid">
                  {loading ? "Registrando..." : "Confirmar rechazo"}
                </button>
              </div>
            </form>
      </Modal>}
    </div>
  );
}
