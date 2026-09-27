import { render, screen, waitFor } from "@testing-library/react";
import { api } from "@/lib/api";
import MapPage from "./page";
import * as L from "leaflet";
const router = { push: jest.fn() };
jest.mock("next/navigation", () => ({ useRouter: () => router }));
jest.mock("@/components/dashboard/Navbar", () => ({ Navbar: () => <nav>Mapa</nav> }));
jest.mock("leaflet", () => {
 const map = { setView: jest.fn().mockReturnThis(), remove: jest.fn() };
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
