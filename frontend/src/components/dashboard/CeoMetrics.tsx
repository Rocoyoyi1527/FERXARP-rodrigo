"use client";
import { DemoNotice } from "@/components/ui/DemoNotice";
import { isDemoNgo } from "@/lib/demo";
import { useEffect, useState } from "react";
import { api, apiErrorMessage, type ImpactMetrics } from "@/lib/api";
import { StatCards } from "@/components/ui/StatCards";
export function CeoMetrics() {
 const [metrics,setMetrics]=useState<ImpactMetrics|null>(null);
 const [hasDemo,setHasDemo]=useState(false);
 const [error,setError]=useState<string|null>(null);
 useEffect(()=>{let active=true;Promise.all([api.getImpactMetrics(),api.getMapPoints()]).then(([data,points])=>{if(active){setMetrics(data);setHasDemo(points.some(p=>isDemoNgo(p.id)));}}).catch(err=>{if(active)setError(apiErrorMessage(err));});return()=>{active=false;};},[]);
 if(error)return <p role="alert" className="notice notice-error">{error}</p>;
 if(!metrics)return <p role="status" className="empty-state">Cargando indicadores de impacto...</p>;
 const rate=metrics.total_donations ? metrics.delivered_donations/metrics.total_donations*100 : null;
 return <>{hasDemo && <DemoNotice/>}<StatCards items={[{label:"Donaciones completadas",value:metrics.delivered_donations,icon:"check"},{label:"CO₂e evitado · estimación (kg)",value:metrics.estimated_co2_saved_kg.toLocaleString(),icon:"chart"},{label:"Beneficiarios · estimación",value:metrics.estimated_beneficiaries.toLocaleString(),icon:"users"},{label:"ONG verificadas",value:metrics.verified_ngos,icon:"shield"}]}/>
 <p className="notice mb-6">Estimación: las cifras de CO₂e, volumen y beneficiarios usan los cálculos actuales del sistema. Sus unidades y metodología están pendientes de validación; no representan impacto certificado. Los registros Demo utilizan datos ficticios y no acreditan operaciones ni beneficiarios reales.</p>
 <section className="panel"><h2 className="panel-title">Avance de las donaciones</h2><p className="panel-subtitle">Estado actual registrado. No hay una serie histórica disponible para mostrar tendencias.</p><StatCards items={[{label:"Total de donaciones",value:metrics.total_donations},{label:"En camino",value:metrics.in_transit_donations,icon:"truck"},{label:"Rechazadas",value:metrics.rejected_donations},{label:"Volumen reportado (kg) · estimación",value:metrics.total_volume_kg.toLocaleString(),icon:"chart"}]}/><div className="flex justify-between text-sm mb-2"><span>Porcentaje entregado</span><strong>{rate===null ? "Sin datos" : `${rate.toFixed(1)}%`}</strong></div>{rate!==null && <progress className="w-full h-3 accent-green-600" value={rate} max={100} aria-label="Porcentaje de donaciones entregadas"/>}{metrics.total_donations===0 && <p className="empty-state">Aún no hay donaciones para calcular el avance.</p>}</section></>;
}
