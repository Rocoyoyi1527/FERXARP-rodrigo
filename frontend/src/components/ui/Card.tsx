import type { ReactNode } from "react";
export function Card({ title, subtitle, children, className = "" }: { title?: string; subtitle?: string; children: ReactNode; className?: string }) {
 return <div className={`panel ${className}`}>{title && <h2 className="panel-title">{title}</h2>}{subtitle && <p className="panel-subtitle">{subtitle}</p>}{children}</div>;
}
