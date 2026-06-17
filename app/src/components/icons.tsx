import type { SVGProps } from "react";

// Tutarlı 20px stroke ikon seti — tek harf yer tutucular yerine.
type IconProps = SVGProps<SVGSVGElement>;

function base(props: IconProps) {
  return {
    width: 20,
    height: 20,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.8,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    ...props,
  };
}

export function WorkspaceIcon(props: IconProps) {
  return (
    <svg {...base(props)}>
      <path d="M4 5h6l2 2h8v11a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1Z" />
    </svg>
  );
}

export function SearchIcon(props: IconProps) {
  return (
    <svg {...base(props)}>
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.2-3.2" />
    </svg>
  );
}

export function AskIcon(props: IconProps) {
  return (
    <svg {...base(props)}>
      <path d="M21 12a8 8 0 0 1-11.5 7.2L4 20l1-4.3A8 8 0 1 1 21 12Z" />
      <path d="M9.5 9.5a2.5 2.5 0 0 1 4.2 1.6c0 1.7-2.2 2-2.2 3.4" />
      <path d="M11.5 17h.01" />
    </svg>
  );
}

export function GraphIcon(props: IconProps) {
  return (
    <svg {...base(props)}>
      <circle cx="6" cy="7" r="2.2" />
      <circle cx="18" cy="6" r="2.2" />
      <circle cx="12" cy="17" r="2.2" />
      <path d="M7.7 8.4 10.6 15M16.4 7.6 13.4 15M8 7.2h8" />
    </svg>
  );
}

export function AgentsIcon(props: IconProps) {
  return (
    <svg {...base(props)}>
      <rect x="5" y="8" width="14" height="11" rx="2" />
      <path d="M12 8V5M9 3.5h6" />
      <circle cx="9.5" cy="13" r="1" />
      <circle cx="14.5" cy="13" r="1" />
    </svg>
  );
}

export function AuraModeIcon(props: IconProps) {
  return (
    <svg {...base(props)}>
      <path d="M13 3 5 13h5l-1 8 8-10h-5l1-8Z" />
    </svg>
  );
}

export function SettingsIcon(props: IconProps) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 13a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-2.9 1.2V21a2 2 0 1 1-4 0v-.2a1.7 1.7 0 0 0-2.9-1.1l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0-1.1-2.9H3a2 2 0 1 1 0-4h.2a1.7 1.7 0 0 0 1.1-2.9l-.1-.1A2 2 0 1 1 7 4.2l.1.1a1.7 1.7 0 0 0 1.9.3H9a1.7 1.7 0 0 0 1-1.6V3a2 2 0 1 1 4 0v.2a1.7 1.7 0 0 0 2.9 1.1l.1-.1A2 2 0 1 1 19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9V9a1.7 1.7 0 0 0 1.6 1H21a2 2 0 1 1 0 4h-.2a1.7 1.7 0 0 0-1.4.9Z" />
    </svg>
  );
}

// Sol üstteki marka simgesi (bağlı düğümler / "ikinci beyin")
export function BrandMark(props: IconProps) {
  return (
    <svg {...base({ width: 24, height: 24, ...props })}>
      <circle cx="12" cy="12" r="3.2" fill="currentColor" stroke="none" />
      <circle cx="5" cy="6" r="1.8" />
      <circle cx="19" cy="6" r="1.8" />
      <circle cx="6" cy="18" r="1.8" />
      <circle cx="18" cy="18" r="1.8" />
      <path d="M6.4 7.3 9.6 10.6M17.6 7.3 14.4 10.6M7.2 16.7 10 13.7M16.8 16.7 14 13.7" />
    </svg>
  );
}
