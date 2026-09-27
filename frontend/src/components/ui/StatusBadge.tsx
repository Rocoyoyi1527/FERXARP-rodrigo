import type { DonationState, RequestState } from "@/lib/api";
export function StatusBadge({ status, requestStatus }: { status: DonationState; requestStatus?: RequestState }) {
  const labels: Record<DonationState, string> = { en_acopio: "Disponible", reservado: requestStatus === "aprobada" ? "Lista para salida" : "Reservada", en_transito: "En camino", entregado: "Entregada", rechazado: "Rechazada" };
  return <span className={`badge ${status === "reservado" ? "badge-warning" : status === "en_transito" ? "badge-info" : status === "rechazado" ? "badge-danger" : ""}`}>{labels[status]}</span>;
}
