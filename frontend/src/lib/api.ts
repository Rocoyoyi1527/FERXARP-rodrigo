const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8000";

export type UserRole = "empresa" | "ong" | "admin" | "ceo";
export type DonationState = "en_acopio" | "reservado" | "en_transito" | "entregado" | "rechazado";
export type RequestState = "pendiente" | "aprobada";
export type ScanAction = "salida" | "entrega" | "rechazo";

const HTTP_MESSAGES: Record<number, string> = {
  400: "Datos inválidos. Revisa la información e inténtalo de nuevo.",
  401: "La sesión ha expirado. Inicia sesión de nuevo.",
  403: "No tienes permiso para realizar esta operación.",
  404: "El recurso ya no está disponible.",
  409: "La operación ya no es válida en el estado actual. Actualiza los datos.",
  500: "Ocurrió un error interno. Inténtalo más tarde.",
};

export class ApiError extends Error {
  constructor(public readonly status: number, message?: string) {
    super(message ?? HTTP_MESSAGES[status] ?? "No se pudo completar la operación.");
    this.name = "ApiError";
  }
}

export function apiErrorMessage(error: unknown): string {
  return error instanceof ApiError ? error.message : "No se pudo conectar con el servidor.";
}

export function isApiError(error: unknown, status: number): boolean {
  return error instanceof ApiError && error.status === status;
}

// --- TIPOS Y CONTRATOS DE DATOS ---

export interface UserClaims {
  sub: string;
  email?: string;
  role: UserRole;
  exp: number;
}

export interface DonationItem {
  id: string;
  user_id?: string;
  title: string;
  description: string | null;
  quantity: number;
  status: DonationState;
  assigned_ngo_id?: string | null;
}

export interface FeedDonationItem {
  id: string;
  title: string;
  description: string | null;
  quantity: number;
  status: DonationState;
  donor_email: string;
  created_at?: string;
}

export interface ShipmentItem {
  id: string;
  donation_id: string;
  title: string;
  description: string | null;
  quantity: number;
  donation_status: DonationState;
  request_status: RequestState;
  donor_email: string;
  ngo_name: string;
  assigned_ngo_id: string | null;
  completed_at: string | null;
  rejection_reason: string | null;
  created_at: string | null;
}

export interface ScoredMatch {
  ngo_id: string;
  ngo_name: string;
  distance_km: number;
  semantic_similarity: number;
  final_score: number;
  ai_reasoning: string | null;
  ai_priority: string | null;
}

export interface ScanResult {
  donation_id: string;
  previous_status: DonationState | null;
  new_status: DonationState;
  message: string;
}

export interface MapPoint {
  id: string;
  name: string;
  point_type: string;
  latitude: number;
  longitude: number;
  status?: string;
  details: string;
}

export interface ImpactMetrics {
  total_donations: number;
  delivered_donations: number;
  in_transit_donations: number;
  rejected_donations: number;
  total_volume_kg: number;
  estimated_co2_saved_kg: number;
  estimated_beneficiaries: number;
  verified_ngos: number;
}

export interface SeedSummary {
  companies_seeded: number;
  ngos_seeded: number;
  donations_seeded: number;
  ai_evaluations?: { donation_title: string; recommended_ngo: string; score: number; priority: string; reasoning: string }[];
}

export type ScanPayload =
  | { donation_id: string; action: "salida" | "entrega"; notes?: string }
  | { donation_id: string; action: "rechazo"; rejection_reason: string; notes?: string };

// --- CLIENTE HTTP ---

async function request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
  const token = typeof window !== "undefined" ? localStorage.getItem("fexarp_token") : null;

  const headers: HeadersInit = {
    "Content-Type": "application/json",
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...options.headers,
  };

  const response = await fetch(`${API_URL}${endpoint}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    throw new ApiError(response.status);
  }

  if (response.status === 204 || response.headers.get("content-length") === "0") {
    return {} as T;
  }

  const contentType = response.headers.get("content-type");
  if (contentType && contentType.includes("application/json")) {
    return response.json();
  }

  const rawText = await response.text();
  return { message: rawText } as T;
}

// --- SERVICIOS DE LA PLATAFORMA ---

export const api = {
  // Autenticación
  login: async (data: { email: string; password: string }) => {
    try {
      return await request<{ token: string }>("/api/auth/login", { method: "POST", body: JSON.stringify(data) });
    } catch (error) {
      if (isApiError(error, 401)) throw new ApiError(401, "Credenciales incorrectas.");
      throw error;
    }
  },

  register: (data: { email: string; password: string; role: "empresa" | "ong" }) =>
    request<{ token: string; message: string }>("/api/auth/register", { method: "POST", body: JSON.stringify(data) }),

  getMe: () => request<UserClaims>("/api/auth/me"),

  // Donaciones
  createDonation: (data: { title: string; description?: string; quantity: number }) =>
    request<DonationItem>("/api/donations", { method: "POST", body: JSON.stringify(data) }),

  listDonations: () => request<DonationItem[]>("/api/donations"),

  getMatches: (id: string) => request<ScoredMatch[]>(`/api/donations/${id}/matches`),

  getDonationFeed: () => request<FeedDonationItem[]>("/api/donations/feed"),

  requestDonation: (donationId: string) =>
    request<void>(`/api/donations/${donationId}/request`, { method: "POST" }),

  // Control de Envíos y Asignaciones
  getShipments: () => request<ShipmentItem[]>("/api/donations/shipments"),

  approveRequest: (donationId: string) =>
    request<void>(`/api/donations/${donationId}/approve`, { method: "POST" }),

  // Logística y Ubicaciones
  shipmentAction: (data: ScanPayload) =>
    request<ScanResult>("/api/scanner/scan", { method: "POST", body: JSON.stringify(data) }),

  getMapPoints: () => request<MapPoint[]>("/api/scanner/map-points"),

  // Métricas del CEO
  getImpactMetrics: () => request<ImpactMetrics>("/api/metrics/summary"),

  // Ingesta de Datos Reales (Veracruz)
  seedVeracruzData: () =>
    request<SeedSummary>(
      "/api/seed/veracruz",
      { method: "POST" }
    ),
};
