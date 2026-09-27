"use client";
import { useState } from "react";
import Link from "next/link";
import type { DonationItem } from "@/lib/api";
import { StatusBadge } from "@/components/ui/StatusBadge";
export function DonationList({ donations, onSelectMatching }: { donations: DonationItem[]; onSelectMatching: (id: string) => void }) {
 const [selected,setSelected]=useState<string|null>(null);
 return <section id="donaciones" className="panel"><div className="card-head mb-5"><div><h2 className="panel-title">Mis donaciones</h2><p className="panel-subtitle mb-0">Consulta su estado y encuentra organizaciones compatibles.</p></div><span className="badge">{donations.length} lotes</span></div>
 {donations.length === 0 ? <div className="empty-state">Todavía no has publicado donaciones. Usa el formulario para crear la primera.</div> : <div className="space-y-4">{donations.map(d=><article key={d.id} className="donation-card" data-selected={selected===d.id}><div className="card-head"><h3>{d.title}</h3><StatusBadge status={d.status}/></div><p className="card-meta">{d.description || "Sin descripción adicional."}</p><p className="card-meta"><strong>{d.quantity}</strong> unidades</p><div className="flex flex-wrap gap-2 mt-4"><button className="btn btn-secondary" onClick={()=>{setSelected(d.id);onSelectMatching(d.id);}}>Ver coincidencias</button><Link className="btn btn-ghost" href="/shipments">Ver envíos</Link></div></article>)}</div>}</section>;
}
