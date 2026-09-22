import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, type Screenshot } from "../../lib/ipc";

/** Smaller than this is a click, not a drag: it takes the whole monitor instead of a sliver. */
const MIN_REGION = 8;

interface Point {
  x: number;
  y: number;
}

/**
 * The frozen monitor, filling its own window: drag a rectangle to pick a region, Enter takes the
 * whole monitor, Escape throws it away.
 *
 * The window sits exactly over the monitor the picture came from, so one CSS pixel of this page is
 * one pixel of the picture — after the browser's own scaling, which `scale` undoes when cutting.
 */
export function RegionOverlay() {
  const { t } = useTranslation();
  const [shot, setShot] = useState<Screenshot | null>(null);
  const [from, setFrom] = useState<Point | null>(null);
  const [to, setTo] = useState<Point | null>(null);
  const image = useRef<HTMLImageElement>(null);

  useEffect(() => {
    void api.screenshotRegionImage().then(setShot);
  }, []);

  /** Hands back the given part of the picture, in the picture's own pixels. */
  const pick = useCallback((left: number, top: number, width: number, height: number) => {
    const source = image.current;
    if (!source) return;
    const canvas = document.createElement("canvas");
    canvas.width = Math.round(width);
    canvas.height = Math.round(height);
    const context = canvas.getContext("2d");
    if (!context) return;
    context.drawImage(source, left, top, width, height, 0, 0, canvas.width, canvas.height);
    void api.screenshotRegionDone({
      // The prefix belongs to the data URL, not to the picture.
      data: canvas.toDataURL("image/png").split(",")[1] ?? "",
      width: canvas.width,
      height: canvas.height,
    });
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") void api.screenshotRegionCancel();
      if (event.key === "Enter" && shot) pick(0, 0, shot.width, shot.height);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [shot, pick]);

  if (!shot) return <div className="h-full w-full bg-black" />;

  const box = from && to ? rectangle(from, to) : null;

  const finish = () => {
    const source = image.current;
    if (!box || !source) return;
    // The picture is drawn to fit the window; a scaled display needs the factor back.
    const scale = shot.width / source.clientWidth;
    if (box.width < MIN_REGION || box.height < MIN_REGION) {
      pick(0, 0, shot.width, shot.height);
    } else {
      pick(box.left * scale, box.top * scale, box.width * scale, box.height * scale);
    }
  };

  return (
    <div
      className="relative h-full w-full cursor-crosshair overflow-hidden bg-black select-none"
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        setFrom({ x: e.clientX, y: e.clientY });
        setTo({ x: e.clientX, y: e.clientY });
      }}
      onPointerMove={(e) => from && setTo({ x: e.clientX, y: e.clientY })}
      onPointerUp={finish}
    >
      <img
        ref={image}
        src={`data:image/png;base64,${shot.data}`}
        alt=""
        draggable={false}
        onLoad={() => void api.screenshotRegionReady()}
        className="pointer-events-none h-full w-full object-fill"
      />
      {/* Everything outside the selection is dimmed, so the picked part stands out. */}
      <div className={box ? "pointer-events-none absolute inset-0" : "pointer-events-none absolute inset-0 bg-black/40"}>
        {box && (
          <>
            <div className="absolute inset-0 bg-black/40" style={{ clipPath: cutOut(box) }} />
            <div
              className="absolute border border-white/90 shadow-[0_0_0_1px_rgba(0,0,0,0.6)]"
              style={{ left: box.left, top: box.top, width: box.width, height: box.height }}
            />
            <div
              className="absolute rounded bg-black/75 px-1.5 py-0.5 font-mono text-caption text-white"
              style={{ left: box.left, top: Math.max(0, box.top - 22) }}
            >
              {Math.round(box.width)} × {Math.round(box.height)}
            </div>
          </>
        )}
      </div>
      {!box && (
        <p className="pointer-events-none absolute inset-x-0 bottom-10 text-center text-body text-white/90">
          <span className="rounded-lg bg-black/70 px-3 py-1.5">{t("screenshot.regionHint")}</span>
        </p>
      )}
    </div>
  );
}

interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

function rectangle(from: Point, to: Point): Box {
  return {
    left: Math.min(from.x, to.x),
    top: Math.min(from.y, to.y),
    width: Math.abs(to.x - from.x),
    height: Math.abs(to.y - from.y),
  };
}

/** Dims the whole window except `box`: the outer ring as one path with a hole in it. */
function cutOut(box: Box): string {
  const right = box.left + box.width;
  const bottom = box.top + box.height;
  return `polygon(0 0, 100% 0, 100% 100%, 0 100%, 0 0, ${box.left}px ${box.top}px, ${box.left}px ${bottom}px, ${right}px ${bottom}px, ${right}px ${box.top}px, ${box.left}px ${box.top}px)`;
}
