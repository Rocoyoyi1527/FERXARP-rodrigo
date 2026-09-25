"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { api, apiErrorMessage } from "@/lib/api";
import { Input } from "@/components/ui/Input";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";

export default function LoginPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);

    try {
      // 1. Petición POST a /api/auth/login en Rust (puerto 8000)
      const res = await api.login({ email, password });

      // 2. Almacenar el JWT para las peticiones autenticadas
      localStorage.setItem("fexarp_token", res.token);

      // 3. Redirigir al panel principal
      router.push("/dashboard");
    } catch (error) {
      setError(apiErrorMessage(error));
    } finally {
      setLoading(false);
    }
  };

  return (
    <main className="min-h-screen bg-neutral-950 text-neutral-100 flex items-center justify-center p-4">
      <Card
        title="Iniciar Sesión"
        subtitle="Acceso a la plataforma Fexarp"
        className="max-w-md w-full shadow-2xl"
      >
        {error && (
          <div className="mb-4 text-xs text-rose-400 bg-rose-950/40 border border-rose-800/50 p-2.5 rounded">
            {error}
          </div>
        )}

        <form onSubmit={handleLogin} className="space-y-4">
          <Input
            label="Correo electrónico"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="contacto@organizacion.com"
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

          <Button type="submit" disabled={loading} className="w-full">
            {loading ? "Validando credenciales..." : "Ingresar"}
          </Button>

          <p className="text-center text-xs text-neutral-400 pt-2">
            ¿No tienes cuenta?{" "}
            <Link href="/register" className="text-neutral-200 underline hover:text-white">
              Regístrate aquí
            </Link>
          </p>
        </form>
      </Card>
    </main>
  );
}
