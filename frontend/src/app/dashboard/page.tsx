"use client";
import { DemoNotice } from "@/components/ui/DemoNotice";
import { isDemoDonation } from "@/lib/demo";
import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { api, apiErrorMessage, isApiError, type DonationItem, type ScoredMatch, type ShipmentItem, type UserClaims } from "@/lib/api";
import { Navbar } from "@/components/dashboard/Navbar";
import { DonationForm } from "@/components/dashboard/DonationForm";
import { DonationList } from "@/components/dashboard/DonationList";
import { GardenGraph } from "@/components/dashboard/GardenGraph";
import { OngCanopy } from "@/components/dashboard/OngCanopy";
import { CeoMetrics } from "@/components/dashboard/CeoMetrics";
import { AdminAudit } from "@/components/dashboard/AdminAudit";
import { StatCards } from "@/components/ui/StatCards";
import { Icon } from "@/components/ui/Icon";
export default function DashboardPage() {
 const router=useRouter();
 const [user,setUser]=useState<UserClaims|null>(null);
 const [donations,setDonations]=useState<DonationItem[]>([]);
 const [shipments,setShipments]=useState<ShipmentItem[]>([]);
 const [selected,setSelected]=useState<DonationItem|null>(null);
 const [matches,setMatches]=useState<ScoredMatch[]>([]);
 const [matching,setMatching]=useState(false);
 const [loading,setLoading]=useState(true);
 const [error,setError]=useState<string|null>(null);
 const requestId=useRef(0);
 const logout=()=>{localStorage.removeItem("fexarp_token");router.push("/login");};
 useEffect(()=>{
  let active=true;
  if(!localStorage.getItem("fexarp_token")){router.push("/login");return;}
  api.getMe().then(async claims=>{
   if(!active)return;setUser(claims);
   if(claims.role === "empresa") {
    const [list,requests]=await Promise.all([api.listDonations(),api.getShipments()]);
    if(active){setDonations(list);setShipments(requests);}
   }
  }).catch(err=>{if(!active)return;setError(apiErrorMessage(err));if(isApiError(err,401)){localStorage.removeItem("fexarp_token");router.push("/login");}}).finally(()=>{if(active)setLoading(false);});
  return()=>{active=false;};
 },[router]);
 const refresh=async()=>{setDonations(await api.listDonations());setShipments(await api.getShipments());};
 const select=async(id:string)=>{
  const current=++requestId.current;setSelected(donations.find(d=>d.id===id)??null);setMatches([]);setMatching(true);setError(null);
  try{const results=await api.getMatches(id);if(current===requestId.current)setMatches(results);}
  catch(err){if(current===requestId.current)setError(apiErrorMessage(err));}
  finally{if(current===requestId.current)setMatching(false);}
 };
 if(loading)return <div className="auth-page" role="status">Cargando tu espacio de trabajo...</div>;
 const titles={empresa:"Cada donación cuenta.",ong:"Encuentra lo que tu comunidad necesita.",admin:"Organizaciones con confianza.",ceo:"El impacto de compartir."};
 return <main className="app-page"><Navbar user={user} onLogout={logout}/><div className="page-content"><header className="page-heading"><div><p className="eyebrow">Tu red, en movimiento</p><h1>{user ? titles[user.role] : "Inicio"}</h1><p>Gestiona tus actividades y acompaña cada entrega, de principio a fin.</p></div>{user?.role === "empresa" && <a href="#nueva-donacion" className="btn btn-primary"><Icon name="plus"/>Nueva donación</a>}{user?.role === "ong" && <a href="#donaciones" className="btn btn-primary">Explorar donaciones</a>}</header>
 {error && <p role="alert" className="notice notice-error mb-5">{error}</p>}
 {user?.role === "ceo" ? <CeoMetrics/> : user?.role === "admin" ? <AdminAudit/> : user?.role === "ong" ? <OngCanopy/> : user?.role === "empresa" ? <>
 {donations.some(d => isDemoDonation(d.id)) && <DemoNotice/>}
 <StatCards items={[{label:"Donaciones activas",value:donations.filter(d=>["en_acopio","reservado","en_transito"].includes(d.status)).length,icon:"box"},{label:"Solicitudes pendientes",value:shipments.filter(s=>s.donation_status==="reservado"&&s.request_status==="pendiente").length,icon:"users"},{label:"Listas para salida",value:shipments.filter(s=>s.donation_status==="reservado"&&s.request_status==="aprobada").length,icon:"truck"},{label:"Entregadas",value:donations.filter(d=>d.status==="entregado").length,icon:"check"}]}/>
 <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 items-start"><div id="nueva-donacion"><DonationForm onDonationCreated={refresh}/></div><div className="lg:col-span-2"><DonationList donations={donations} onSelectMatching={select}/></div></div><div className="mt-6"><GardenGraph donation={selected} matches={matches} isLoading={matching}/></div>
 </> : null}</div></main>;
}
