"use client";

import { useEffect, useState } from "react";

interface NgoAuditItem {
  id: string;
  name: string;
  email: string;
  needs_description?: string;
  is_verified: boolean;
  created_at: string;
}

export function AdminAudit() {
  const [ngos, setNgos] = useState<NgoAuditItem[]>([]);
  const [loading, setLoading] = useState(true);

  const loadNgos = async () => {
    const token = localStorage.getItem("fexarp_token");
    try {
      const res = await fetch("http://localhost:8000/api/auth/ngos", {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (res.ok) {
        const data = await res.json();
        setNgos(data);
      }
    } catch (err) {
      console.error("Error al cargar organizaciones:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    let active = true;
    const token = localStorage.getItem("fexarp_token");
    fetch("http://localhost:8000/api/auth/ngos", {
      headers: { Authorization: `Bearer ${token}` },
    })
      .then((response) => response.ok ? response.json() as Promise<NgoAuditItem[]> : [])
      .then((data) => { if (active) setNgos(data); })
      .catch((error: unknown) => console.error("Error al cargar organizaciones:", error))
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, []);

  const toggleVerify = async (id: string) => {
    const token = localStorage.getItem("fexarp_token");
    try {
      const res = await fetch(`http://localhost:8000/api/auth/ngos/${id}/verify`, {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` },
      });
      if (res.ok) {
        loadNgos();
      }
    } catch (err) {
      console.error("Error al alternar estado:", err);
    }
  };

  if (loading) {
    return (
      <div className="h-64 flex items-center justify-center border border-garden-border rounded-2xl bg-garden-surface/60 font-mono text-xs text-garden-sage">
        Auditoria de credenciales y organizaciones en curso...
      </div>
    );
  }

  return (
    <div className="border border-garden-border bg-garden-surface/80 backdrop-blur-md rounded-2xl p-6 shadow-garden-glow">
      <div className="flex justify-between items-center mb-6">
        <div>
          <h2 className="text-base font-semibold text-white tracking-tight flex items-center gap-2">
            <span className="w-2.5 h-2.5 rounded-full bg-garden-amber animate-pulse" />
            Panel de Seguridad y Auditoría de ONGs (Admin TI)
          </h2>
          <p className="text-xs text-garden-sage mt-0.5">
            Certificación de organizaciones receptoras para mitigar riesgos de opacidad o desvío de recursos
          </p>
        </div>
        <span className="text-[11px] font-mono px-3 py-1 rounded-full bg-garden-dark border border-garden-border text-garden-leaf">
          {ngos.length} Organizaciones Registradas
        </span>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-left text-xs">
          <thead>
            <tr className="border-b border-garden-border/80 text-garden-sage font-mono uppercase text-[10px] tracking-wider">
              <th className="pb-3 px-3">Organización Social</th>
              <th className="pb-3 px-3">Contacto Oficial</th>
              <th className="pb-3 px-3">Demanda Operativa</th>
              <th className="pb-3 px-3">Estado de Acreditación</th>
              <th className="pb-3 px-3 text-right">Acción</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-garden-border/40">
            {ngos.map((ngo) => (
              <tr key={ngo.id} className="hover:bg-garden-dark/40 transition">
                <td className="py-3.5 px-3 font-semibold text-white">{ngo.name}</td>
                <td className="py-3.5 px-3 font-mono text-neutral-400">{ngo.email}</td>
                <td className="py-3.5 px-3 text-garden-sage max-w-xs truncate">
                  {ngo.needs_description || "Sin descripción declarada"}
                </td>
                <td className="py-3.5 px-3">
                  {ngo.is_verified ? (
                    <span className="text-[10px] font-mono px-2.5 py-0.5 rounded-full bg-emerald-950/40 border border-emerald-500/50 text-garden-sprout flex items-center gap-1 w-fit">
                      <span className="w-1.5 h-1.5 rounded-full bg-garden-leaf" />
                      Verificada y Confiable
                    </span>
                  ) : (
                    <span className="text-[10px] font-mono px-2.5 py-0.5 rounded-full bg-amber-950/40 border border-amber-600/50 text-amber-300 flex items-center gap-1 w-fit">
                      <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
                      Pendiente de Validación
                    </span>
                  )}
                </td>
                <td className="py-3.5 px-3 text-right">
                  <button
                    onClick={() => toggleVerify(ngo.id)}
                    className={`text-[11px] font-mono px-3 py-1.5 rounded-xl border transition cursor-pointer ${
                      ngo.is_verified
                        ? "bg-garden-dark border-rose-800/60 text-rose-300 hover:bg-rose-950/50"
                        : "bg-garden-leaf text-garden-obsidian border-garden-leaf font-semibold hover:opacity-90"
                    }`}
                  >
                    {ngo.is_verified ? "Revocar Acreditación" : "Acreditar ONG"}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
