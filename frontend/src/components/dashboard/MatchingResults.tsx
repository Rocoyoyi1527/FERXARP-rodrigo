import { Card } from "@/components/ui/Card";

interface MatchItem {
  ngo_id: string;
  ngo_name: string;
  final_score: number;
  distance_km: number | null;
  semantic_similarity: number;
}

export function MatchingResults({ matches }: { matches: MatchItem[] }) {
  if (matches.length === 0) return null;

  return (
    <Card title="Ranking de Prioridad" subtitle="Scoring adaptado (Distancia Haversine + ChromaDB)">
      <div className="space-y-2 mt-2">
        {matches.map((m) => (
          <div
            key={m.ngo_id}
            className="flex justify-between items-center text-xs border-b border-neutral-800/80 py-2.5 last:border-b-0"
          >
            <div>
              <span className="font-semibold text-white">{m.ngo_name}</span>
              <span className="text-neutral-500 ml-2">({m.distance_km === null ? "distancia no disponible" : `${m.distance_km} km de distancia`})</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-neutral-400">Score:</span>
              <span className="font-mono font-bold text-emerald-400 text-sm">
                {m.final_score} / 100
              </span>
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}
