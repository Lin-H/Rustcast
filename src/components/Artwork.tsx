import { useEffect, useState } from "preact/hooks";
import { convertFileSrc } from "@tauri-apps/api/core";
import { cacheArtwork } from "../services/tauri";

interface ArtworkProps {
  src: string | null;
  fallbackSrc: string | null;
  alt: string;
  className?: string;
  placeholderClassName?: string;
}

interface ResolvedSource {
  /** 首选展示地址（缓存命中时为 asset 协议，否则为远程 URL）。 */
  primary: string | null;
  /** 缓存失败后的远程回落地址。 */
  remote: string | null;
}

/** 解析封面地址：远程 URL 先走本地磁盘缓存，失败时回落远程。 */
function useResolvedSource(src: string | null): ResolvedSource {
  const [cachedPath, setCachedPath] = useState<string | null>(null);
  const [cacheFailed, setCacheFailed] = useState(false);

  useEffect(() => {
    setCachedPath(null);
    setCacheFailed(false);
    if (src === null || !src.startsWith("http")) {
      return;
    }

    let cancelled = false;
    void cacheArtwork(src)
      .then((path) => {
        if (!cancelled && path !== null) {
          setCachedPath(path);
        } else if (!cancelled) {
          setCacheFailed(true);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setCacheFailed(true);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [src]);

  if (src === null) {
    return { primary: null, remote: null };
  }
  if (cachedPath !== null) {
    return { primary: convertFileSrc(cachedPath), remote: src };
  }
  if (cacheFailed) {
    return { primary: src, remote: null };
  }
  // 缓存请求进行中：直接用远程地址先行展示，缓存命中后由下次渲染替换。
  return { primary: src, remote: src };
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
  const resolved = useResolvedSource(src);

  useEffect(() => {
    setPrimaryFailed(false);
  }, [resolved.primary]);

  useEffect(() => {
    setFallbackFailed(false);
  }, [fallbackSrc]);

  const activeSrc = !primaryFailed ? resolved.primary : fallbackSrc;
  const failed =
    (resolved.primary !== null && primaryFailed && (fallbackSrc === null || fallbackFailed)) ||
    (resolved.primary === null && fallbackSrc === null);

  if (activeSrc === null || failed) {
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
      loading="lazy"
      decoding="async"
      className={`shrink-0 bg-card object-cover ${className}`}
      onError={() => {
        if (!primaryFailed && resolved.primary !== null) {
          setPrimaryFailed(true);
        } else {
          setFallbackFailed(true);
        }
      }}
    />
  );
}
