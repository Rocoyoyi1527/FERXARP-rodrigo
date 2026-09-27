import type { ButtonHTMLAttributes } from "react";
interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> { variant?: "primary" | "secondary" | "outline" | "ghost"; loading?: boolean; }
export function Button({ children, variant = "primary", loading = false, className = "", disabled, ...props }: ButtonProps) {
 return <button className={`btn btn-${variant} ${className}`} disabled={disabled || loading} aria-busy={loading || undefined} {...props}>{loading ? "Cargando..." : children}</button>;
}
