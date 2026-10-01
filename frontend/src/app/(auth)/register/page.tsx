"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { api, apiErrorMessage } from "@/lib/api";
import { Input } from "@/components/ui/Input";
import { Button } from "@/components/ui/Button";
import { AuthIntro } from "@/components/ui/AuthIntro";
import { Icon } from "@/components/ui/Icon";
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
      // 1. Registro en PostgreSQL a través del backend en Rust
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
    <main className="auth-page"><div className="auth-wrap"><AuthIntro />
      <Card title="Crear Cuenta" subtitle="Suma tu organización a la Red Verde Solidaria." className="max-w-md w-full">
        {error && (
          <div role="alert" className="notice notice-error mb-4">
            {error}
          </div>
        )}
        <form onSubmit={handleRegister} className="space-y-4">
          <Input
            label="Correo corporativo / institucional"
            type="email"
            autoComplete="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="ejemplo@organizacion.com"
            required
          />
          <Input
            label="Contraseña"
            type="password"
            autoComplete="new-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="••••••••••••"
            required
          />

          <div className="space-y-1">
            <label htmlFor="organization-role" className="field-label">Tipo de Organización</label>
            <div className="role-choice">{(["empresa", "ong"] as const).map((value) => <button key={value} type="button" aria-pressed={role === value} onClick={() => setRole(value)}><Icon name={value === "empresa" ? "box" : "users"}/><strong className="block mt-2">{value === "empresa" ? "Empresa" : "ONG"}</strong><span>{value === "empresa" ? "Quiero donar" : "Quiero recibir"}</span></button>)}</div>
            <select
              id="organization-role"
              value={role}
              onChange={(e) => setRole(e.target.value as "empresa" | "ong")}
              className="field"
            >
              <option value="empresa">Empresa Donante</option>
              <option value="ong">ONG / Organización Benéfica</option>
            </select>
          </div>

          <Button type="submit" disabled={loading} className="w-full">
            {loading ? "Creando cuenta..." : "Registrarse"}
          </Button>

          <p className="text-center text-sm text-garden-sage pt-2">
            ¿Ya tienes cuenta?{" "}
            <Link href="/login" className="text-garden-leaf font-semibold underline">
              Inicia sesión
            </Link>
          </p>
        </form>
      </Card>
    </div></main>
  );
}
