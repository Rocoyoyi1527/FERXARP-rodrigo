import { Card } from "@/components/ui/Card";

interface MatchItem {
  ngo_id: string;
  ngo_name: string;
  final_score: number;
  distance_km: number | null;
  semantic_similarity: number;
}

export function MatchingResults({ matches }: { matches: MatchItem[] }) {
  if (matches.length === 0) return <p className="empty-state">No hay organizaciones compatibles para esta donación.</p>;

  return (
    <Card title="Organizaciones compatibles" subtitle="Resultados de compatibilidad para esta donación">
      <div className="space-y-2 mt-2">
        {matches.map((m) => (
          <div
            key={m.ngo_id}
            className="flex justify-between items-center text-xs border-b border-garden-border py-2.5 last:border-b-0"
          >
            <div>
              <span className="font-semibold text-garden-text">{m.ngo_name}</span>
              <span className="text-garden-sage ml-2">({m.distance_km === null ? "distancia no disponible" : `${m.distance_km} km de distancia`})</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-garden-sage">Compatibilidad:</span>
              <span className="font-sans font-bold text-garden-leaf text-sm">
                {m.final_score} / 100
              </span>
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}
