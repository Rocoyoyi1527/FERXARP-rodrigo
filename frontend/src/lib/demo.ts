// Exact fixture identities from backend/src/api/seed.rs::demo_id and seed/data.rs.
// Do not infer demo provenance from a title, email domain, or arbitrary UUID prefix.
function fixtureIds(kind: number, count: number): Set<string> {
  return new Set(Array.from({ length: count }, (_, index) =>
    `fea00000-0000-4000-8000-${kind.toString(16).padStart(4, "0")}${index.toString(16).padStart(8, "0")}`));
}
const donations = fixtureIds(4, 20);
const ngos = fixtureIds(3, 10);
export const isDemoDonation = (id: string) => donations.has(id);
export const isDemoNgo = (id: string) => ngos.has(id);

// Display labels derived from the six seed account identifiers, not legal company names.
const companyLabels: Record<string, string> = {
  "alimentos-veracruz@demo.ferxarp.invalid": "Alimentos Veracruz Demo",
  "tecnologia-boca@demo.ferxarp.invalid": "Tecnología Boca Demo",
  "mobiliario-centro@demo.ferxarp.invalid": "Mobiliario Centro Demo",
  "suministros-medellin@demo.ferxarp.invalid": "Suministros Medellín Demo",
  "herramientas-puerto@demo.ferxarp.invalid": "Herramientas Puerto Demo",
  "textiles-veracruz@demo.ferxarp.invalid": "Textiles Veracruz Demo",
};
export function companyLabel(donationId: string, email: string): string {
  return isDemoDonation(donationId) ? companyLabels[email] ?? email : email;
}
export function donationTitle(id: string, title: string): string {
  return isDemoDonation(id) ? title.replace(/^Demo · /, "") : title;
}
export function donationDescription(id: string, text: string | null): string | null {
  return isDemoDonation(id)
    ? text?.replace(/^DEMO FICTICIA · no acredita operaciones ni impacto real\. /, "") ?? null
    : text;
}
export function ngoDescription(id: string, text: string | undefined): string | undefined {
  return isDemoNgo(id)
    ? text?.replace(/^DEMO FICTICIA · coordenadas aproximadas y verificación simulada, sin vínculo con organizaciones reales\. /, "")
    : text;
}
