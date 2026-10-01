"use client";

import { DemoNotice, DemoBadge } from "@/components/ui/DemoNotice";
import { isDemoDonation, donationTitle, donationDescription, companyLabel } from "@/lib/demo";
import { useEffect, useState } from "react";
import { api, apiErrorMessage, FeedDonationItem, isApiError, ShipmentItem } from "@/lib/api";
import { StatCards } from "@/components/ui/StatCards";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { StockScanner } from "@/components/scanner/StockScanner";
import { useRouter } from "next/navigation";

export function OngCanopy() {
  const [feed, setFeed] = useState<FeedDonationItem[]>([]);
  const router = useRouter();
  const [myDonations, setMyDonations] = useState<ShipmentItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [requestingId, setRequestingId] = useState<string | null>(null);
  const [feedbackMsg, setFeedbackMsg] = useState<{ id: string; text: string; error?: boolean } | null>(null);

  const loadData = async () => {
    try {
      const [feedData, assignedData] = await Promise.all([
        api.getDonationFeed(),
        api.getShipments(),
      ]);
      setFeed(feedData);
      setMyDonations(assignedData);
    } catch (error) {
      if (isApiError(error, 401)) {
        localStorage.removeItem("fexarp_token");
        router.replace("/login");
      } else {
        setFeedbackMsg({ id: "global", text: apiErrorMessage(error), error: true });
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    let active = true;
    Promise.all([api.getDonationFeed(), api.getShipments()])
      .then(([available, requested]) => {
        if (!active) return;
        setFeed(available);
        setMyDonations(requested);
      })
      .catch((error: unknown) => {
        if (!active) return;
        if (isApiError(error, 401)) {
          localStorage.removeItem("fexarp_token");
          router.replace("/login");
        } else {
          setFeedbackMsg({ id: "global", text: apiErrorMessage(error), error: true });
        }
      })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [router]);

  const handleRequest = async (id: string) => {
    setRequestingId(id);
    setFeedbackMsg(null);
    try {
      await api.requestDonation(id);
      setFeedbackMsg({ id: "global", text: "Solicitud registrada. La Empresa debe aprobarla antes de la salida." });
      await loadData();
    } catch (error) {
      setFeedbackMsg({ id: "global", text: apiErrorMessage(error), error: true });
      if (isApiError(error, 409)) await loadData();
      if (isApiError(error, 401)) {
        localStorage.removeItem("fexarp_token");
        router.replace("/login");
      }
    } finally {
      setRequestingId(null);
    }
  };

  if (loading) {
    return (
      <div className="h-64 flex items-center justify-center border border-garden-border rounded-2xl bg-garden-surface/60 font-sans text-xs text-garden-sage">
        Consultando donaciones disponibles en la zona...
      </div>
    );
  }

  return (
    <>{(feed.some(d => isDemoDonation(d.id)) || myDonations.some(d => isDemoDonation(d.donation_id))) && <DemoNotice/>}<StatCards items={[{label:"Donaciones disponibles",value:feed.length},{label:"Mis solicitudes",value:myDonations.filter(d=>d.donation_status==="reservado").length,icon:"users"},{label:"En camino",value:myDonations.filter(d=>d.donation_status==="en_transito").length,icon:"truck"},{label:"Recibidas",value:myDonations.filter(d=>d.donation_status==="entregado").length,icon:"check"}]}/><div id="donaciones" className="grid grid-cols-1 lg:grid-cols-12 gap-6">
      {feedbackMsg && (
        <p role="status" className={`lg:col-span-12 rounded-xl border px-3 py-2 text-xs ${feedbackMsg.error ? "border-red-200 text-red-700" : "border-garden-emerald text-garden-sprout"}`}>
          {feedbackMsg.text}
        </p>
      )}
      {/* Columna Izquierda: Donaciones Disponibles */}
      <div className="lg:col-span-7 space-y-4">
        <div className="border border-garden-border bg-garden-surface/80  rounded-2xl p-5 shadow-garden-glow">
          <div className="flex justify-between items-center mb-4">
            <div>
              <h2 className="text-sm font-semibold text-garden-text tracking-tight flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-garden-leaf " />
                Donaciones Disponibles para Solicitar
              </h2>
              <p className="text-sm text-garden-sage">
                Excedentes publicados por empresas listos para ser canalizados a tu comunidad
              </p>
            </div>
            <span className="text-xs font-sans px-2.5 py-0.5 rounded-full bg-garden-dark border border-garden-border text-garden-sprout">
              {feed.length} Disponibles
            </span>
          </div>

          {feed.length === 0 ? (
            <div className="text-center py-8 border border-dashed border-garden-border/60 rounded-xl bg-garden-dark/30">
              <p className="text-xs text-garden-sage">No hay donaciones disponibles.</p>
              <p className="text-sm text-garden-sage mt-1">Vuelve más tarde para consultar nuevas publicaciones de las empresas.</p>
            </div>
          ) : (
            <div className="space-y-3">
              {feed.map((item) => (
                <div
                  key={item.id}
                  className="border border-garden-border bg-garden-dark/70 rounded-xl p-4 flex flex-col justify-between gap-3 hover:border-garden-border-glow transition"
                >
                  <div className="flex flex-col sm:flex-row justify-between sm:items-center gap-3">
                    <div>
                      <h3 className="text-base font-semibold text-garden-text">{isDemoDonation(item.id) && <DemoBadge/>}{donationTitle(item.id, item.title)}</h3><StatusBadge status={item.status}/>{item.created_at && <p className="card-meta">Publicado: {new Date(item.created_at).toLocaleDateString("es-MX")}</p>}
                      {item.description && (
                        <p className="text-sm text-garden-sage mt-0.5">{donationDescription(item.id, item.description)}</p>
                      )}
                      <p className="text-xs font-sans text-garden-sage mt-1">
                        Empresa: <span className="text-garden-text">{companyLabel(item.id, item.donor_email)}</span> | Volumen:{" "}
                        <span className="text-garden-leaf font-bold">{item.quantity}</span> unidades
                      </p>
                    </div>

                    <button
                      type="button"
                      disabled={requestingId !== null}
                      onClick={() => handleRequest(item.id)}
                      className="btn btn-primary shrink-0"
                    >
                      {requestingId === item.id ? (
                        <span className="font-sans text-xs">Apartando...</span>
                      ) : (
                        <>
                          <span>Solicitar Donación</span>
                          <span className="text-sm">→</span>
                        </>
                      )}
                    </button>
                  </div>

                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Columna Derecha: Donaciones Solicitadas y Recepción */}
      <div className="lg:col-span-5 space-y-4">
        <div className="border border-garden-border bg-garden-surface/80  rounded-2xl p-5 shadow-garden-glow">
          <div className="flex justify-between items-center mb-4">
            <div>
              <h2 className="text-sm font-semibold text-garden-text tracking-tight">Mis Solicitudes y Entregas</h2>
              <p className="text-sm text-garden-sage">Insumos apartados para recepción física y control de entrega</p>
            </div>
            <span className="text-xs font-sans px-2 py-0.5 rounded-full bg-garden-dark border border-garden-border text-garden-leaf">
              {myDonations.length} Lotes
            </span>
          </div>

          {myDonations.length === 0 ? (
            <div className="text-center py-8 border border-dashed border-garden-border/60 rounded-xl bg-garden-dark/30">
              <p className="text-xs text-garden-sage">Aún no has solicitado donaciones.</p>
            </div>
          ) : (
            <div className="space-y-3">
              {myDonations.map((d) => (
                <div key={d.id} className="border border-garden-border bg-garden-dark/80 rounded-xl p-3.5">
                  <div className="flex justify-between items-start">
                    <div>
                      <h3 className="text-base font-semibold text-garden-text">{isDemoDonation(d.donation_id) && <DemoBadge/>}{donationTitle(d.donation_id, d.title)}</h3>{d.description && <p className="card-meta">{donationDescription(d.donation_id, d.description)}</p>}
                      <p className="text-sm font-sans text-garden-sage mt-0.5">
                        Cantidad: <span className="text-garden-sprout">{d.quantity}</span> unidades
                      </p>
                    </div>
                    <StatusBadge status={d.donation_status} requestStatus={d.request_status}/>
                  </div>
                  {d.completed_at && <p className="mt-2 text-sm text-garden-sage">Finalizada: {new Date(d.completed_at).toLocaleString("es-MX")}</p>}
                  {d.rejection_reason && <p className="mt-2 text-sm text-red-700">Motivo: {d.rejection_reason}</p>}
                  {d.donation_status === "en_transito" && (
                    <StockScanner
                      donationId={d.donation_id}
                      actions={["entrega", "rechazo"]}
                      onStatusChanged={loadData}
                      onFeedback={(text, error) => setFeedbackMsg(text ? { id: "global", text, error } : null)}
                    />
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div></>
  );
}
