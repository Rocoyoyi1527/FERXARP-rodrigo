"use client";
import { useCallback, useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { api, apiErrorMessage, isApiError, type MapPoint, type UserClaims } from "@/lib/api";
import { createMapPopupContent, isDemoWarehouse } from "@/lib/mapPopup";
import { requestUserPosition, type UserPosition } from "@/lib/geolocation";
import { Navbar } from "@/components/dashboard/Navbar";
import { Icon } from "@/components/ui/Icon";
import type { Map as LeafletMap, Marker } from "leaflet";
const REFERENCE: [number,number] = [19.1738,-96.1342];
export default function MapPage() {
 const router=useRouter();
 const container=useRef<HTMLDivElement>(null);
 const mapRef=useRef<LeafletMap|null>(null);
 const locationMarker=useRef<Marker|null>(null);
 const mounted=useRef(false);
 const [user,setUser]=useState<UserClaims|null>(null);
 const [points,setPoints]=useState<MapPoint[]>([]);
 const [activeShipments,setActiveShipments]=useState<number|null>(null);
 const [loading,setLoading]=useState(true);
 const [ready,setReady]=useState(false);
 const [error,setError]=useState<string|null>(null);
 const [position,setPosition]=useState<UserPosition|null>(null);
 const [locationStatus,setLocationStatus]=useState("Tu ubicación se solicita al navegador; puedes rechazar el permiso.");
 const [locating,setLocating]=useState(false);
 const locate=useCallback(async()=>{
  setLocating(true);
  try { const coords=await requestUserPosition();if(mounted.current){setPosition(coords);setLocationStatus("Tu ubicación fue obtenida por el navegador.");} }
  catch { if(mounted.current){setPosition(null);setLocationStatus("No se pudo obtener tu ubicación.");} }
  finally {if(mounted.current)setLocating(false);}
 },[]);
 useEffect(()=>{
  mounted.current=true;
  if(!localStorage.getItem("fexarp_token")){router.push("/login");return;}
  Promise.all([api.getMe(),api.getMapPoints()]).then(([claims,data])=>{if(mounted.current){setUser(claims);setPoints(data);}}).catch(err=>{if(mounted.current){setError(apiErrorMessage(err));if(isApiError(err,401)){localStorage.removeItem("fexarp_token");router.push("/login");}}}).finally(()=>{if(mounted.current)setLoading(false);});
  api.getShipments().then(data=>{if(mounted.current)setActiveShipments(data.filter(s=>s.donation_status==="en_transito").length);}).catch(()=>{/* El conteo queda no disponible si falla esta petición. */});
  return()=>{mounted.current=false;};
 },[router]);
 useEffect(()=>{
  if(loading||!container.current)return;
  let active=true;
  import("leaflet").then(L=>{
   if(!active||!container.current)return;
   const map=L.map(container.current,{zoomControl:false}).setView(REFERENCE,12);mapRef.current=map;
   L.control.zoom({position:"bottomright"}).addTo(map);
   L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png",{maxZoom:19,attribution:'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'}).addTo(map);
   const acopio=points.find(p=>p.point_type==="acopio");
   points.forEach(p=>{
    if(!Number.isFinite(p.latitude)||!Number.isFinite(p.longitude)||Math.abs(p.latitude)>90||Math.abs(p.longitude)>180)return;
    const warehouse=p.point_type==="acopio";
    const marker=L.marker([p.latitude,p.longitude],{title:isDemoWarehouse(p)?"Almacén demo":p.name,icon:L.divIcon({className:"",html:`<div class="map-pin ${warehouse?"map-pin-warehouse":""}">${warehouse?"▣":"♥"}</div>`,iconSize:[30,30],iconAnchor:[15,15]})}).addTo(map);
    marker.bindPopup(createMapPopupContent(p),{className:"custom-popup"});
    if(!warehouse&&acopio)L.polyline([[acopio.latitude,acopio.longitude],[p.latitude,p.longitude]],{color:"#16a34a",weight:2,opacity:.35,dashArray:"5,8"}).addTo(map);
   });
   setReady(true);
   void locate();
  }).catch(()=>{if(active)setError("No se pudo cargar el mapa. Recarga la página para intentarlo de nuevo.");});
  return()=>{active=false;mapRef.current?.remove();mapRef.current=null;locationMarker.current=null;};
 },[loading,points,locate]);
 useEffect(()=>{
  if(!ready)return;
  let active=true;
  locationMarker.current?.remove();locationMarker.current=null;
  if(position)void import("leaflet").then(L=>{
   if(!active||!mapRef.current)return;
   const label=document.createElement("strong");label.textContent="Tu ubicación";
   locationMarker.current=L.marker([position.latitude,position.longitude],{title:"Tu ubicación",icon:L.divIcon({className:"",html:'<div class="map-pin map-pin-user">●</div>',iconSize:[30,30],iconAnchor:[15,15]})}).addTo(mapRef.current).bindPopup(label);
   mapRef.current.setView([position.latitude,position.longitude],13);
  });
  return()=>{active=false;};
 },[position,ready]);
 return <main className="app-page"><Navbar user={user} activePage="map" onLogout={()=>{localStorage.removeItem("fexarp_token");router.push("/login");}}/><div className="page-content"><header className="page-heading"><div><p className="eyebrow">Cerca de tu comunidad</p><h1>Mapa de organizaciones</h1><p>Vista de referencia: Veracruz. Los puntos corresponden a las coordenadas registradas en la plataforma.</p></div><div className="flex flex-wrap gap-2"><button disabled={!ready||locating} className="btn btn-primary" onClick={()=>void locate()}><Icon name="location"/>{locating?"Obteniendo ubicación...":"Usar mi ubicación"}</button><button disabled={!ready} className="btn btn-secondary" onClick={()=>mapRef.current?.setView(REFERENCE,12)}>Ver Veracruz</button></div></header>
 {error && <p role="alert" className="notice notice-error">{error}</p>}
 <p role="status" className="text-sm text-garden-sage">{locationStatus}</p>
 <div className="map-summary"><span><strong>{points.filter(p=>p.point_type==="acopio").length}</strong> almacenes</span><span><strong>{points.filter(p=>p.point_type==="ong").length}</strong> ONG</span><span><strong>{activeShipments??"No disponible"}</strong> envíos activos visibles para tu cuenta</span></div>
 <div className="map-canvas">{loading && <div className="absolute inset-0 grid place-items-center">Cargando mapa...</div>}<div ref={container} className="h-full w-full" aria-label="Mapa de puntos de acopio y ONG"/></div>
 <div className="map-legend" aria-label="Leyenda del mapa"><span><i className="map-pin map-pin-user">●</i>Usuario (solo con ubicación real)</span><span><i className="map-pin map-pin-warehouse">▣</i>Almacén / punto de acopio</span><span><i className="map-pin">♥</i>ONG</span><span>┄ Conexiones de referencia, no rutas de transporte</span></div>
 {!loading && points.length===0 && <p className="empty-state">No hay puntos registrados para mostrar.</p>}
 <p className="text-sm text-garden-sage">El almacén demo es un punto fijo de referencia. Los envíos no tienen seguimiento GPS en este mapa.</p></div></main>;
}
