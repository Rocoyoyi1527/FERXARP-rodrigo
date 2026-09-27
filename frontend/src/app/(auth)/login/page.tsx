"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { api, apiErrorMessage } from "@/lib/api";
import { Input } from "@/components/ui/Input";
import { Button } from "@/components/ui/Button";
import { AuthIntro } from "@/components/ui/AuthIntro";
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
    <main className="auth-page"><div className="auth-wrap"><AuthIntro />
      <Card
        title="Iniciar Sesión"
        subtitle="Bienvenido. Continúa ayudando desde tu cuenta."
        className="max-w-md w-full shadow-2xl"
      >
        {error && (
          <div role="alert" className="notice notice-error mb-4">
            {error}
          </div>
        )}

        <form onSubmit={handleLogin} className="space-y-4">
          <Input
            label="Correo electrónico"
            type="email"
            autoComplete="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="contacto@organizacion.com"
            required
          />

          <Input
            label="Contraseña"
            type="password"
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="••••••••••••"
            required
          />

          <Button type="submit" disabled={loading} className="w-full">
            {loading ? "Validando credenciales..." : "Ingresar"}
          </Button>

          <p className="text-center text-sm text-garden-sage pt-2">
            ¿No tienes cuenta?{" "}
            <Link href="/register" className="text-garden-leaf font-semibold underline">
              Regístrate aquí
            </Link>
          </p>
        </form>
      </Card>
    </div></main>
  );
}
