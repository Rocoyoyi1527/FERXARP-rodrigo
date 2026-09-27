import { Icon } from "./Icon";
export function StatCards({ items }: { items: { label: string; value: number | string; icon?: "box" | "truck" | "check" | "users" | "chart" | "shield" }[] }) {
  return <div className="stat-grid">{items.map(item => <div className="stat-card" key={item.label}><div className="stat-icon"><Icon name={item.icon ?? "box"} /></div><p className="stat-value">{item.value}</p><p className="stat-label">{item.label}</p></div>)}</div>;
}
