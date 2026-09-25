"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { api, apiErrorMessage } from "@/lib/api";
import { Input } from "@/components/ui/Input";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";

export default function RegisterPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<"empresa" | "ong">("empresa");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);

    try {
      // 1. Registro en Supabase a través del backend en Rust
      await api.register({ email, password, role });

      // 2. Login inmediato para obtener el token JWT
      const auth = await api.login({ email, password });
      localStorage.setItem("fexarp_token", auth.token);

      // 3. Redirección al panel principal
      router.push("/dashboard");
    } catch (error) {
      setError(apiErrorMessage(error));
    } finally {
      setLoading(false);
    }
  };

  return (
    <main className="min-h-screen bg-neutral-950 text-neutral-100 flex items-center justify-center p-4">
      <Card title="Crear Cuenta" subtitle="Registro de entidad en Fexarp" className="max-w-md w-full">
        {error && (
          <div className="mb-4 text-xs text-rose-400 bg-rose-950/40 border border-rose-800/50 p-2.5 rounded">
            {error}
          </div>
        )}
        <form onSubmit={handleRegister} className="space-y-4">
          <Input
            label="Correo corporativo / institucional"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="ejemplo@organizacion.com"
            required
          />
          <Input
            label="Contraseña"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="••••••••••••"
            required
          />

          <div className="space-y-1">
            <label className="block text-xs text-neutral-400">Tipo de Organización</label>
            <select
              value={role}
              onChange={(e) => setRole(e.target.value as "empresa" | "ong")}
              className="w-full bg-neutral-950 border border-neutral-800 rounded px-3 py-2 text-sm text-neutral-100 focus:outline-none focus:border-neutral-600 cursor-pointer"
            >
              <option value="empresa">Empresa Donante</option>
              <option value="ong">ONG / Organización Benéfica</option>
            </select>
          </div>

          <Button type="submit" disabled={loading} className="w-full">
            {loading ? "Creando cuenta..." : "Registrarse"}
          </Button>

          <p className="text-center text-xs text-neutral-400 pt-2">
            ¿Ya tienes cuenta?{" "}
            <Link href="/login" className="text-neutral-200 underline hover:text-white">
              Inicia sesión
            </Link>
          </p>
        </form>
      </Card>
    </main>
  );
}
