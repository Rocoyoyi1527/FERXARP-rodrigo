import React, { useId } from "react";

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export function Input({ label, error, className = "", ...props }: InputProps) {
  const generatedId = useId();
  const inputId = props.id ?? generatedId;
  return (
    <div className="w-full space-y-1.5">
      {label && (
        <label htmlFor={inputId} className="block text-[11px] font-mono uppercase tracking-wider text-garden-sage">
          {label}
        </label>
      )}
      <input
        id={inputId}
        className={`w-full bg-garden-dark border border-garden-border rounded-xl px-3.5 py-2.5 text-xs text-white placeholder:text-garden-sage/40 focus:outline-none focus:border-garden-leaf focus:ring-1 focus:ring-garden-leaf transition font-sans ${
          error ? "border-rose-500/70 focus:border-rose-500 focus:ring-rose-500/50" : ""
        } ${className}`}
        {...props}
      />
      {error && (
        <p className="text-[11px] font-mono text-rose-400 mt-1">{error}</p>
      )}
    </div>
  );
}
