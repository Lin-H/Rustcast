import { useEffect, useState } from "preact/hooks";

interface ArtworkProps {
  src: string | null;
  fallbackSrc: string | null;
  alt: string;
  className?: string;
  placeholderClassName?: string;
}

export function Artwork({
  src,
  fallbackSrc,
  alt,
  className = "h-14 w-14 rounded-lg",
  placeholderClassName = "text-sm",
}: ArtworkProps) {
  const [primaryFailed, setPrimaryFailed] = useState(false);
  const [fallbackFailed, setFallbackFailed] = useState(false);

  useEffect(() => {
    setPrimaryFailed(false);
  }, [src]);

  useEffect(() => {
    setFallbackFailed(false);
  }, [fallbackSrc]);

  const activeSrc = !primaryFailed && src ? src : fallbackSrc;
  const failed = src ? primaryFailed && fallbackFailed : fallbackFailed;

  if (!activeSrc || failed) {
    return (
      <div
        className={`flex shrink-0 items-center justify-center bg-card text-secondary ${className}`}
        aria-label={alt}
      >
        <span className={placeholderClassName}>{alt.slice(0, 1) || "播"}</span>
      </div>
    );
  }

  return (
    <img
      key={activeSrc}
      src={activeSrc}
      alt={alt}
      className={`shrink-0 bg-card object-cover ${className}`}
      onError={() => {
        if (!primaryFailed && src) {
          setPrimaryFailed(true);
        } else {
          setFallbackFailed(true);
        }
      }}
    />
  );
}
