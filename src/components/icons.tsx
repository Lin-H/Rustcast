interface IconProps {
  className?: string;
}

export function BrandIcon({ className = "h-5 w-5" }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <circle cx="12" cy="11" r="2.4" fill="currentColor" />
      <path
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        d="M8.2 15.8a5.4 5.4 0 1 1 7.6 0M5.4 18.6a9.3 9.3 0 1 1 13.2 0M12 13.5V21"
      />
    </svg>
  );
}

export function PlayIcon({ className = "h-5 w-5" }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M8 5.14v13.72c0 .83.92 1.33 1.62.89l10.8-6.86a1.05 1.05 0 0 0 0-1.78L9.62 4.25A1.05 1.05 0 0 0 8 5.14Z" />
    </svg>
  );
}

export function PauseIcon({ className = "h-5 w-5" }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <rect x="6" y="5" width="4" height="14" rx="1.4" />
      <rect x="14" y="5" width="4" height="14" rx="1.4" />
    </svg>
  );
}

export function VolumeIcon({ className = "h-5 w-5" }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        fill="currentColor"
        d="M13 4.7v14.6c0 .86-.99 1.34-1.66.8L6.9 16.4H4a2 2 0 0 1-2-2v-4.8a2 2 0 0 1 2-2h2.9l4.44-3.7c.67-.55 1.66-.07 1.66.8Z"
      />
      <path
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        d="M16.5 9.5a4 4 0 0 1 0 5M19 7a7.5 7.5 0 0 1 0 10"
      />
    </svg>
  );
}
