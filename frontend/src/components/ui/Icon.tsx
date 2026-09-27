import type { SVGProps } from "react";
const paths = {
  leaf: "M20 4C9 2 3 8 5 15c2 7 15 5 15-11ZM4 21 15 10",
  home: "m3 10 9-7 9 7v11h-6v-7H9v7H3Z",
  box: "m3 7 9-4 9 4-9 4-9-4Zm0 0v10l9 4 9-4V7M12 11v10M7 5l10 4",
  truck: "M3 5h11v12H3ZM14 9h4l3 4v4h-7M7 20a2 2 0 1 0 0-4 2 2 0 0 0 0 4Zm11 0a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z",
  map: "m3 5 6-2 6 2 6-2v16l-6 2-6-2-6 2V5Zm6-2v16m6-14v16",
  check: "m5 12 4 4L19 6",
  users: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm8-7a4 4 0 0 1 0 7m5 10v-2a4 4 0 0 0-3-4",
  chart: "M3 3v18h18M7 16v-5m5 5V7m5 9V4",
  shield: "m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3Zm-4 9 3 3 5-6",
  menu: "M4 6h16M4 12h16M4 18h16",
  plus: "M12 4v16M4 12h16",
  location: "M12 3a7 7 0 0 0-7 7c0 5 7 11 7 11s7-6 7-11a7 7 0 0 0-7-7Zm0 10a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z",
};
export function Icon({ name, ...props }: SVGProps<SVGSVGElement> & { name: keyof typeof paths }) {
  return <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" {...props}><path d={paths[name]} /></svg>;
}
