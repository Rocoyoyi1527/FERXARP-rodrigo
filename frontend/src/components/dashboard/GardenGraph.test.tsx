import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { api, type DonationItem, type ScoredMatch } from "@/lib/api";
import { GardenGraph } from "./GardenGraph";

const donation: DonationItem = {
  id: "test-donation", title: "Alimentos", description: null, quantity: 5, status: "en_acopio",
};

const match: ScoredMatch = {
  ngo_id: "ngo-1", ngo_name: '<img src=x onerror=alert(1)>', distance_km: null,
  semantic_similarity: 0.9, lexical_score: 80, vector_score: 90, final_score: 85,
  ai_reasoning: null, ai_priority: null,
};

describe("GardenGraph", () => {
  afterEach(() => jest.restoreAllMocks());

  it("shows a deduplicated map of organizations when no donation is selected", async () => {
    jest.spyOn(api, "getMapPoints").mockResolvedValue([
      { id: "1", name: "ONG Prueba", point_type: "ong", latitude: 1, longitude: 1, details: "Necesidades" },
      { id: "2", name: "ONG Prueba", point_type: "ong", latitude: 2, longitude: 2, details: "Repetida" },
      { id: "3", name: "Acopio", point_type: "acopio", latitude: 3, longitude: 3, details: "Centro" },
    ]);
    render(<GardenGraph donation={null} matches={[]} />);
    await waitFor(() => expect(screen.getByText("ONG Prueba")).toBeInTheDocument());
    expect(screen.getAllByText("ONG Prueba")).toHaveLength(1);
    expect(screen.getByText("Centro de Acopio")).toBeInTheDocument();
  });

  it("keeps match names as text and displays score details", () => {
    const { container } = render(<GardenGraph donation={donation} matches={[match]} />);
    expect(container.querySelector("img")).toBeNull();
    expect(screen.getAllByText('<img src=x onerror=alert(1)>').length).toBeGreaterThan(0);
    expect(screen.getByText(/Prioridad: 85.0\/100/)).toBeInTheDocument();
    expect(screen.getByText(/Distancia: no disponible/)).toBeInTheDocument();
    fireEvent.click(screen.getByText(/Prioridad: 85.0\/100/));
  });

  it("renders an empty state and a loading state", () => {
    const empty = render(<GardenGraph donation={donation} matches={[]} />);
    expect(screen.getByText("No se detectaron organizaciones conectadas")).toBeInTheDocument();
    empty.unmount();
    render(<GardenGraph donation={donation} matches={[]} isLoading />);
    expect(screen.getByText(/Puntuando afinidad/)).toBeInTheDocument();
  });
});
