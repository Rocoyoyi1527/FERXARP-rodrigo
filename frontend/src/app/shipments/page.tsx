"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { api, apiErrorMessage, isApiError, ShipmentItem, UserClaims } from "@/lib/api";
import { Navbar } from "@/components/dashboard/Navbar";
import { StockScanner } from "@/components/scanner/StockScanner";

type Notice = { text: string; error: boolean };

export default function ShipmentsPage() {
  const router = useRouter();
  const [user, setUser] = useState<UserClaims | null>(null);
  const [shipments, setShipments] = useState<ShipmentItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [approvingId, setApprovingId] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const approvalInFlight = useRef(false);

  useEffect(() => {
    let active = true;
    if (!localStorage.getItem("fexarp_token")) {
      router.replace("/login");
      return;
    }
    Promise.all([api.getMe(), api.getShipments()])
      .then(([claims, data]) => {
        if (!active) return;
        setUser(claims);
        setShipments(data);
      })
      .catch((error: unknown) => {
        if (!active) return;
        if (isApiError(error, 401)) {
          localStorage.removeItem("fexarp_token");
          router.replace("/login");
        } else {
          setNotice({ text: apiErrorMessage(error), error: true });
        }
      })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [router]);

  const refresh = async () => {
    try {
      setShipments(await api.getShipments());
    } catch (error) {
      if (isApiError(error, 401)) {
        localStorage.removeItem("fexarp_token");
        router.replace("/login");
      } else {
        setNotice({ text: apiErrorMessage(error), error: true });
      }
    }
  };

  const handleApprove = async (donationId: string) => {
    if (approvalInFlight.current) return;
    approvalInFlight.current = true;
    setApprovingId(donationId);
    setNotice(null);
    try {
      await api.approveRequest(donationId);
      setNotice({ text: "Solicitud aprobada. La donación sigue reservada hasta registrar su salida.", error: false });
      await refresh();
    } catch (error) {
      setNotice({ text: apiErrorMessage(error), error: true });
      if (isApiError(error, 409)) await refresh();
      if (isApiError(error, 401)) {
        localStorage.removeItem("fexarp_token");
        router.replace("/login");
      }
    } finally {
      approvalInFlight.current = false;
      setApprovingId(null);
    }
  };

  const pending = shipments.filter((s) => s.donation_status === "reservado" && s.request_status === "pendiente");
  const ready = shipments.filter((s) => s.donation_status === "reservado" && s.request_status === "aprobada" && s.assigned_ngo_id);
  const inTransit = shipments.filter((s) => s.donation_status === "en_transito");
  const finished = shipments.filter((s) => s.donation_status === "entregado" || s.donation_status === "rechazado");

  if (loading) return <div className="min-h-screen bg-garden-obsidian flex items-center justify-center text-xs font-mono text-garden-sage">Cargando solicitudes y envíos...</div>;

  const card = (s: ShipmentItem, stage: "pending" | "ready" | "transit" | "finished") => (
    <article key={s.id} className="rounded-xl border border-garden-border bg-garden-dark p-4 space-y-2">
      <div className="flex items-start justify-between gap-2">
        <h4 className="text-xs font-semibold text-white">{s.title}</h4>
        <span className="text-[10px] font-mono text-garden-sprout whitespace-nowrap">
          {stage === "pending" ? "Pendiente" : stage === "ready" ? "Aprobada" : stage === "transit" ? "En camino" : s.donation_status === "entregado" ? "Entregada" : "Rechazada"}
        </span>
      </div>
      <p className="text-[11px] text-garden-sage">{s.quantity} unidades · ONG: <span className="text-garden-leaf">{s.ngo_name}</span></p>
      <p className="text-[10px] text-neutral-400">Donante: {s.donor_email}</p>
      {stage === "pending" && user?.role === "empresa" && (
        <button type="button" disabled={approvingId !== null} onClick={() => void handleApprove(s.donation_id)}
          className="w-full mt-2 rounded-xl bg-garden-emerald px-3 py-2 text-xs font-semibold text-garden-obsidian disabled:opacity-50">
          {approvingId === s.donation_id ? "Aprobando..." : "Aprobar solicitud"}
        </button>
      )}
      {stage === "ready" && (
        <>
          <p className="text-[11px] text-amber-300">Aprobada — pendiente de salida física.</p>
          {(user?.role === "empresa" || user?.role === "admin") && (
            <StockScanner donationId={s.donation_id} actions={["salida"]} onStatusChanged={refresh}
              onFeedback={(text, error) => setNotice(text ? { text, error } : null)} />
          )}
        </>
      )}
      {stage === "transit" && (user?.role === "ong" || user?.role === "admin") && (
        <StockScanner donationId={s.donation_id} actions={["entrega", "rechazo"]} onStatusChanged={refresh}
          onFeedback={(text, error) => setNotice(text ? { text, error } : null)} />
      )}
      {stage === "finished" && (
        <>
          {s.completed_at && <p className="text-[11px] text-garden-sage">Finalizada: {new Date(s.completed_at).toLocaleString("es-MX")}</p>}
          {s.donation_status === "rechazado" && s.rejection_reason && (
            <p className="rounded-lg border border-rose-900/40 bg-rose-950/20 p-2 text-[11px] text-rose-300">Motivo: {s.rejection_reason}</p>
          )}
        </>
      )}
    </article>
  );

  const column = (title: string, items: ShipmentItem[], stage: "pending" | "ready" | "transit" | "finished") => (
    <section className="rounded-2xl border border-garden-border bg-garden-surface/70 p-5 shadow-garden-glow">
      <h3 className="mb-4 text-sm font-semibold text-white">{title} ({items.length})</h3>
      {items.length ? <div className="space-y-3">{items.map((s) => card(s, stage))}</div>
        : <p className="rounded-xl border border-dashed border-garden-border p-6 text-center text-xs text-garden-sage">Sin registros en esta etapa.</p>}
    </section>
  );

  return (
    <main className="min-h-screen bg-garden-obsidian p-8 text-neutral-100">
      <Navbar user={user} onLogout={() => { localStorage.removeItem("fexarp_token"); router.push("/login"); }} />
      <div className="mx-auto max-w-7xl space-y-6">
        <header>
          <h2 className="text-lg font-semibold text-white">Control de Solicitudes y Envíos</h2>
          <p className="mt-1 text-xs text-garden-sage">La aprobación reserva el destino; la salida inicia el traslado.</p>
        </header>
        {notice && <p role="status" className={`rounded-xl border p-3 text-xs ${notice.error ? "border-rose-800 text-rose-300" : "border-garden-emerald text-garden-sprout"}`}>{notice.text}</p>}
        <div className="grid grid-cols-1 gap-5 md:grid-cols-2 xl:grid-cols-4">
          {column("1. Solicitadas", pending, "pending")}
          {column("2. Listas para salida", ready, "ready")}
          {column("3. En camino", inTransit, "transit")}
          {column("4. Finalizadas", finished, "finished")}
        </div>
      </div>
    </main>
  );
}
