import { createMapPopupContent } from "./mapPopup";

describe("map popup", () => {
  it("renders stored name and details as text, not executable HTML", () => {
    const popup = createMapPopupContent({
      id: "test",
      name: '<img src=x onerror=alert(1)>',
      point_type: "ong",
      latitude: 0,
      longitude: 0,
      details: '<script>alert(1)</script>',
    });

    expect(popup.querySelector("img, script")).toBeNull();
    expect(popup.querySelector("strong")?.textContent).toBe('<img src=x onerror=alert(1)>');
    expect(popup.querySelector("p")?.textContent).toBe('<script>alert(1)</script>');
    expect(popup.textContent).toContain("Organización");
  });
});
