"use client";
import { DemoNotice, DemoBadge } from "@/components/ui/DemoNotice";
import { isDemoNgo, ngoDescription } from "@/lib/demo";
import { useEffect, useState } from "react";
import { api, apiErrorMessage, type NgoAuditItem } from "@/lib/api";
import { StatCards } from "@/components/ui/StatCards";
export function AdminAudit() {
 const [ngos,setNgos]=useState<NgoAuditItem[]>([]);
 const [loading,setLoading]=useState(true);
 const [busy,setBusy]=useState<string|null>(null);
 const [error,setError]=useState<string|null>(null);
 const [notice,setNotice]=useState<string|null>(null);
 useEffect(()=>{let active=true;api.listNgos().then(data=>{if(active)setNgos(data);}).catch(err=>{if(active)setError(apiErrorMessage(err));}).finally(()=>{if(active)setLoading(false);});return()=>{active=false;};},[]);
 const toggle=async(ngo:NgoAuditItem)=>{setBusy(ngo.id);setError(null);setNotice(null);try{await api.toggleNgoVerification(ngo.id);setNgos(await api.listNgos());setNotice(ngo.is_verified ? "Verificación revocada." : "ONG verificada correctamente.");}catch(err){setError(apiErrorMessage(err));}finally{setBusy(null);}};
 if(loading)return <p role="status" className="empty-state">Cargando organizaciones...</p>;
 return <>{ngos.some(n => isDemoNgo(n.id)) && <><DemoNotice/><p className="text-sm text-garden-sage mb-4">La verificación de las ONG Demo es simulada; sus coordenadas son aproximadas.</p></>}<StatCards items={[{label:"Pendientes de verificación",value:ngos.filter(n=>!n.is_verified).length,icon:"users"},{label:"ONG verificadas",value:ngos.filter(n=>n.is_verified).length,icon:"shield"},{label:"Organizaciones registradas",value:ngos.length,icon:"box"}]}/>
 {error && <p role="alert" className="notice notice-error mb-4">{error}</p>}{notice && <p role="status" className="notice mb-4">{notice}</p>}
 <section className="panel"><h2 className="panel-title">Administración de organizaciones</h2><p className="panel-subtitle">Revisa los datos de cada ONG antes de verificar su participación.</p>{ngos.length===0 ? <p className="empty-state">No hay organizaciones pendientes de verificación ni registradas.</p> : <div className="grid md:grid-cols-2 gap-4">{ngos.map(ngo=><article className="donation-card" key={ngo.id}><div className="card-head"><h3>{isDemoNgo(ngo.id) && <DemoBadge/>}{ngo.name}</h3><span className={`badge ${ngo.is_verified ? "" : "badge-warning"}`}>{ngo.is_verified ? "Verificada" : "Pendiente"}</span></div><p className="card-meta">{ngo.email}</p><p className="card-meta">{ngoDescription(ngo.id, ngo.needs_description) || "Sin necesidades declaradas."}</p><p className="card-meta">Registro: {new Date(ngo.created_at).toLocaleDateString("es-MX")}</p><button disabled={busy!==null} className={`btn mt-4 ${ngo.is_verified ? "btn-danger" : "btn-primary"}`} onClick={()=>void toggle(ngo)}>{busy===ngo.id ? "Guardando..." : ngo.is_verified ? "Revocar" : "Verificar"}</button></article>)}</div>}</section></>;
}
