import { useState } from "react";
import { api, apiErrorMessage, isApiError } from "@/lib/api";
import { useRouter } from "next/navigation";
import { Input } from "@/components/ui/Input";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";

interface DonationFormProps {
  onDonationCreated: () => Promise<void> | void;
}

export function DonationForm({ onDonationCreated }: DonationFormProps) {
  const router = useRouter();
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [quantity, setQuantity] = useState(10);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setSuccess(null);
    try {
      await api.createDonation({
        title,
        description: description || undefined,
        quantity: Number(quantity),
      });
      setTitle("");
      setDescription("");
      await onDonationCreated();
      setSuccess("Donación publicada y disponible para solicitudes.");
    } catch (error) {
      setError(apiErrorMessage(error));
      if (isApiError(error, 401)) {
        localStorage.removeItem("fexarp_token");
        router.push("/login");
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card title="Nueva donación" subtitle="Publica excedentes de stock para ONGs" className="h-fit">
      {error && <div role="alert" className="notice notice-error mb-3">{error}</div>}
      {success && <div role="status" className="notice mb-3">{success}</div>}
      <form onSubmit={handleSubmit} className="space-y-4">
        <Input
          label="Título del lote"
          placeholder="Ej. Cajas de leche pasteurizada"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          required
        />
        <Input
          label="Descripción o requerimientos"
          placeholder="Ej. Lácteos sellados fecha prox. 15 días"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
        <Input
          label="Cantidad de unidades"
          type="number"
          min={1}
          step={1}
          value={quantity}
          onChange={(e) => setQuantity(Number(e.target.value))}
          required
        />
        <Button type="submit" disabled={loading} className="w-full">
          {loading ? "Registrando..." : "Publicar donación"}
        </Button>
      </form>
    </Card>
  );
}
