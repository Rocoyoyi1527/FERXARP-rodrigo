import { companyLabel, donationDescription, donationTitle, isDemoDonation, isDemoNgo, ngoDescription } from "./demo";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const id = "fea00000-0000-4000-8000-000400000000";
const historical = "f5d13322-a795-4ef3-bccc-a36771bd9c07";
const disclaimer = "DEMO FICTICIA · no acredita operaciones ni impacto real. ";
it("cleans only seed donations and preserves historical content even when it looks like demo text", () => {
  expect(donationTitle(id, "Demo · Leche sellada")).toBe("Leche sellada");
  expect(donationDescription(id, disclaimer + "80 cajas.")).toBe("80 cajas.");
  expect(donationDescription(id, null)).toBeNull();
  expect(donationTitle(historical, "Demo · Cajas de leche")).toBe("Demo · Cajas de leche");
  expect(donationDescription(historical, disclaimer + "Original")).toBe(disclaimer + "Original");
  expect(isDemoDonation("fea00000-0000-4000-8000-000400000014")).toBe(false);
  expect(isDemoDonation("fea00000-0000-4000-8000-000300000000")).toBe(false);
});
it("uses seed account labels without relabeling foreign donations or unknown companies", () => {
  const email = "alimentos-veracruz@demo.ferxarp.invalid";
  expect(companyLabel(id, email)).toBe("Alimentos Veracruz Demo");
  expect(companyLabel(historical, email)).toBe(email);
  expect(companyLabel(id, "unknown@example.org")).toBe("unknown@example.org");
});
it("cleans only seeded NGO descriptions", () => {
  const ngo = "fea00000-0000-4000-8000-000300000009";
  const text = "DEMO FICTICIA · coordenadas aproximadas y verificación simulada, sin vínculo con organizaciones reales. Alimentos.";
  expect(isDemoNgo(ngo)).toBe(true);
  expect(isDemoNgo("fea00000-0000-4000-8000-00030000000a")).toBe(false);
  expect(ngoDescription(ngo, text)).toBe("Alimentos.");
  expect(ngoDescription(historical, text)).toBe(text);
});
it("stays aligned with the backend seed namespace and fixture membership", () => {
  const source = readFileSync(resolve(process.cwd(), "../backend/src/api/seed/data.rs"), "utf8");
  const seed = readFileSync(resolve(process.cwd(), "../backend/src/api/seed.rs"), "utf8");
  expect(seed).toContain("0xfea00000_0000_4000_8000_000000000000 | (kind << 32) | index as u128");
  expect(source.match(/title: "Demo · /g)).toHaveLength(20);
  expect(source.match(/email: "/g)).toHaveLength(10);
});
