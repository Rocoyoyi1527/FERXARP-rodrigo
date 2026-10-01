"use client";
import { useState } from "react";
import Link from "next/link";
import { api, apiErrorMessage, isApiError, type UserClaims, type SeedSummary } from "@/lib/api";
import { Icon } from "@/components/ui/Icon";
import { Modal } from "@/components/ui/Modal";
export function Navbar({ user, onLogout, activePage = "dashboard" }: { user: UserClaims | null; onLogout: () => void; activePage?: "dashboard" | "shipments" | "map" }) {
 const [open,setOpen]=useState(false);
 const [confirm,setConfirm]=useState(false);
 const [seeding,setSeeding]=useState(false);
 const [result,setResult]=useState<SeedSummary|null>(null);
 const [error,setError]=useState<string|null>(null);
 const roles = { empresa:"Empresa", ong:"ONG", admin:"Administración", ceo:"Dirección" };
 const seed = async () => {
  setConfirm(false); setSeeding(true); setError(null);
  try { setResult(await api.seedVeracruzData()); }
  catch(err) { setError(apiErrorMessage(err)); if(isApiError(err,401)) onLogout(); }
  finally { setSeeding(false); }
 };
 return <>
 <header className="app-header">
  <div className="header-top">
   <Link href="/dashboard" className="brand" aria-label="FERXARP Inicio"><Icon name="leaf" className="brand-mark"/><div><p className="brand-name">FERXARP</p><p className="brand-tag">Red Verde Solidaria</p></div></Link>
   <div className="account">{user && <div className="account-role"><p className="badge">{roles[user.role]}</p><p className="account-email">{user.email || "Sesión activa"}</p></div>}<button className="btn btn-ghost mobile-menu" aria-label="Abrir navegación" aria-expanded={open} aria-controls="main-navigation" onClick={()=>setOpen(!open)}><Icon name="menu"/></button><button onClick={onLogout} className="btn btn-ghost">Cerrar Sesión</button></div>
  </div>
  <nav id="main-navigation" aria-label="Navegación principal" className="main-nav" data-open={open}>
   <Link href="/dashboard" className="nav-link" aria-current={activePage === "dashboard" ? "page" : undefined}><Icon name={user?.role === "admin" ? "shield" : user?.role === "ceo" ? "chart" : "home"}/>{user?.role === "admin" ? "Administración" : user?.role === "ceo" ? "Impacto" : "Inicio"}</Link>
   {(user?.role === "empresa" || user?.role === "ong") && <Link href="/dashboard#donaciones" className="nav-link"><Icon name="box"/>Donaciones</Link>}
   <Link href="/shipments" className="nav-link" aria-current={activePage === "shipments" ? "page" : undefined}><Icon name="truck"/>Solicitudes y envíos</Link>
   <Link href="/map" className="nav-link" aria-current={activePage === "map" ? "page" : undefined}><Icon name="map"/>Mapa</Link>
   {user?.role === "admin" && <button className="nav-link ml-auto" disabled={seeding} onClick={()=>setConfirm(true)}>{seeding ? "Preparando datos..." : "Cargar datos demo"}</button>}
  </nav>
 </header>
 {error && <p role="alert" className="notice notice-error page-content mt-4">{error}</p>}
 {confirm && <Modal title="Cargar datos de demostración" onClose={()=>setConfirm(false)}><p className="text-sm text-garden-sage mb-5">Se crearán o actualizarán datos ficticios para demostración. Los registros existentes se conservan.</p><div className="flex justify-end gap-2"><button className="btn btn-secondary" onClick={()=>setConfirm(false)}>Cancelar</button><button className="btn btn-primary" onClick={()=>void seed()}>Confirmar carga</button></div></Modal>}
 {result && <Modal title="Datos de demostración preparados" onClose={()=>setResult(null)}><p className="notice">{result.companies_seeded} empresas · {result.ngos_seeded} ONG · {result.donations_seeded} donaciones</p>{result.ai_evaluations?.map((item,i)=><p className="text-sm mt-3" key={i}>{item.donation_title} → {item.recommended_ngo}: {item.reasoning}</p>)}<button className="btn btn-primary mt-5" onClick={()=>window.location.reload()}>Actualizar vista</button></Modal>}
 </>;
}
