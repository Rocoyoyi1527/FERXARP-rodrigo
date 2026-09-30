import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { api } from "@/lib/api";
import MapPage from "./page";
import * as L from "leaflet";
const router = { push: jest.fn() };
jest.mock("next/navigation", () => ({ useRouter: () => router }));
jest.mock("@/components/dashboard/Navbar", () => ({ Navbar: () => <nav>Mapa</nav> }));
jest.mock("leaflet", () => {
 const map = { setView: jest.fn().mockReturnThis(), fitBounds: jest.fn().mockReturnThis(), remove: jest.fn() };
 return { map: jest.fn(()=>map), control: { zoom: jest.fn(()=>({addTo:jest.fn()})) }, tileLayer: jest.fn(()=>({addTo:jest.fn()})), divIcon: jest.fn(options=>options), marker:jest.fn(()=>({addTo:jest.fn().mockReturnThis(),bindPopup:jest.fn().mockReturnThis(),remove:jest.fn()})),polyline:jest.fn(()=>({addTo:jest.fn()})) };
});
const geolocation = { getCurrentPosition: jest.fn() };
beforeEach(()=>{
 jest.clearAllMocks(); localStorage.setItem("fexarp_token","test");
 Object.defineProperty(navigator,"geolocation",{configurable:true,value:geolocation});
 jest.spyOn(api,"getMe").mockResolvedValue({sub:"user",role:"empresa",exp:9999999999});
 jest.spyOn(api,"getMapPoints").mockResolvedValue([{id:"00000000-0000-0000-0000-000000000000",name:"Centro de Acopio Central Fexarp",point_type:"acopio",latitude:19.1738,longitude:-96.1342,details:"Demo"}]);
 jest.spyOn(api,"getShipments").mockResolvedValue([]);
});
afterEach(()=>jest.restoreAllMocks());
test("permiso denegado conserva referencia Veracruz y no crea marcador de usuario",async()=>{
 geolocation.getCurrentPosition.mockImplementation((_ok,fail)=>fail({code:1}));
 render(<MapPage/>);
 expect(await screen.findByText("No se pudo obtener tu ubicación.")).toBeInTheDocument();
 expect(screen.getByText(/Vista de referencia: Veracruz/)).toBeInTheDocument();
 expect(L.marker).toHaveBeenCalledTimes(1);
 expect(L.marker).toHaveBeenCalledWith([19.1738,-96.1342],expect.objectContaining({title:"Almacén demo"}));
});
test("solo las coordenadas del navegador producen el marcador Tu ubicación",async()=>{
 geolocation.getCurrentPosition.mockImplementation(ok=>ok({coords:{latitude:20.5,longitude:-97.1}}));
 render(<MapPage/>);
 await waitFor(()=>expect(L.marker).toHaveBeenCalledWith([20.5,-97.1],expect.objectContaining({title:"Tu ubicación"})));
});
test("sin API geolocation no se inventa un marcador",async()=>{
 Object.defineProperty(navigator,"geolocation",{configurable:true,value:undefined});
 render(<MapPage/>);
 expect(await screen.findByText("No se pudo obtener tu ubicación.")).toBeInTheDocument();
 expect(L.marker).toHaveBeenCalledTimes(1);
});

test("encuadra todas las ONG registradas, incluidas las de Medellín, sin inventar ubicación",async()=>{
 geolocation.getCurrentPosition.mockImplementation((_ok,fail)=>fail({code:1}));
 jest.spyOn(api,"getMapPoints").mockResolvedValue([
  {id:"00000000-0000-0000-0000-000000000000",name:"Almacén demo",point_type:"acopio",latitude:19.1738,longitude:-96.1342,details:"Demo"},
  {id:"medellin",name:"Refugio Demo Medellín",point_type:"ong",latitude:19.066,longitude:-96.157,details:"Demo"},
  {id:"invalid",name:"Coordenadas inválidas",point_type:"ong",latitude:999,longitude:-96,details:"Demo"},
 ]);
 render(<MapPage/>);
 await screen.findByText("No se pudo obtener tu ubicación.");
 const map = jest.mocked(L.map).mock.results[0].value;
 expect(map.fitBounds).toHaveBeenCalledWith([[19.1738,-96.1342],[19.066,-96.157]],{padding:[36,36],maxZoom:12,animate:false});
 expect(L.marker).toHaveBeenCalledTimes(2);
 fireEvent.click(screen.getByRole("button",{name:"Ver Veracruz"}));
 expect(map.fitBounds).toHaveBeenCalledTimes(2);
});
