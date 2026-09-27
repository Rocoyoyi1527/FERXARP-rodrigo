"use client";
import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
export function Modal({ title, children, onClose, busy = false }: { title: string; children: ReactNode; onClose: () => void; busy?: boolean }) {
 const ref = useRef<HTMLDivElement>(null);
 const closeRef = useRef(onClose);
 useEffect(() => { closeRef.current = onClose; }, [onClose]);
 useEffect(() => {
  const previous = document.activeElement as HTMLElement | null;
  const overflow = document.body.style.overflow;
  document.body.style.overflow = "hidden";
  ref.current?.focus();
  const handle = (event: KeyboardEvent) => {
   if (event.key === "Escape" && !busy) closeRef.current();
   if (event.key !== "Tab") return;
   const items = Array.from(ref.current?.querySelectorAll<HTMLElement>('button:not(:disabled), input, textarea, select, a[href], [tabindex="0"]') ?? []);
   const first = items[0], last = items.at(-1);
   if (!first) { event.preventDefault(); return; }
   if (event.shiftKey && (document.activeElement === first || document.activeElement === ref.current)) { event.preventDefault(); last?.focus(); }
   else if (!event.shiftKey && (document.activeElement === last || document.activeElement === ref.current)) { event.preventDefault(); first.focus(); }
  };
  window.addEventListener("keydown", handle);
  return () => { document.body.style.overflow = overflow; window.removeEventListener("keydown", handle); previous?.focus(); };
 }, [busy]);
 return createPortal(<div className="modal-overlay"><div ref={ref} role="dialog" aria-modal="true" aria-label={title} tabIndex={-1} className="modal-panel"><h2 className="panel-title mb-4">{title}</h2>{children}</div></div>,document.body);
}
