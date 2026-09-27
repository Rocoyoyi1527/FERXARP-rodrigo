import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AdminAudit } from "./AdminAudit";
import { api, ApiError } from "@/lib/api";
const ngo={id:"ngo-1",name:"Comunidad",email:"ong@test.invalid",is_verified:false,created_at:"2026-09-26T00:00:00Z"};
afterEach(()=>jest.restoreAllMocks());
test("verifica y revoca con actualización visible del estado",async()=>{
 const list=jest.spyOn(api,"listNgos").mockResolvedValue([ngo]);
 const toggle=jest.spyOn(api,"toggleNgoVerification").mockResolvedValue();
 render(<AdminAudit/>);
 const verify=await screen.findByRole("button",{name:"Verificar"});
 list.mockResolvedValue([{...ngo,is_verified:true}]);
 await userEvent.click(verify);
 expect(await screen.findByRole("button",{name:"Revocar"})).toBeInTheDocument();
 expect(toggle).toHaveBeenCalledWith("ngo-1");
 list.mockResolvedValue([ngo]);
 await userEvent.click(screen.getByRole("button",{name:"Revocar"}));
 expect(await screen.findByText("Verificación revocada.")).toBeInTheDocument();
});
test("un fallo de acceso se comunica y no aparenta una lista válida",async()=>{
 jest.spyOn(api,"listNgos").mockRejectedValue(new ApiError(403));
 render(<AdminAudit/>);
 expect(await screen.findByRole("alert")).toHaveTextContent("No tienes permiso");
});
