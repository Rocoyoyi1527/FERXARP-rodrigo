import { useId, type InputHTMLAttributes } from "react";
interface InputProps extends InputHTMLAttributes<HTMLInputElement> { label?: string; error?: string; }
export function Input({ label, error, className = "", ...props }: InputProps) {
 const id = useId(); const inputId = props.id ?? id;
 return <div><label htmlFor={inputId} className="field-label">{label}</label><input id={inputId} className={`field ${className}`} aria-invalid={Boolean(error)} aria-describedby={error ? `${inputId}-error` : undefined} {...props}/>{error && <p id={`${inputId}-error`} className="text-red-700 text-sm mt-1">{error}</p>}</div>;
}
