import { Archive, Check, ChevronDown, Link2, Loader2, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { createArkAsset, listArkAssets } from "../api";
import type { ArkAsset, ReferenceImageInput } from "../types";

export function SeedanceAssetPicker({
  images,
  maxImages,
  setImages,
}: {
  images: ReferenceImageInput[];
  maxImages: number;
  setImages: Dispatch<SetStateAction<ReferenceImageInput[]>>;
}) {
  const rootRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [assets, setAssets] = useState<ArkAsset[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [sourceUrl, setSourceUrl] = useState("");
  const selectedIds = useMemo(
    () => new Set(images.flatMap((image) => image.assetId ? [image.assetId] : [])),
    [images],
  );

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const nextAssets = await listArkAssets();
      setAssets(nextAssets.filter((asset) => asset.assetType.toLowerCase() === "image"));
    } catch (nextError) {
      setError(String(nextError));
    } finally {
      setLoaded(true);
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open && !loaded && !loading) {
      void refresh();
    }
  }, [loaded, loading, open, refresh]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const closeOutside = (event: PointerEvent) => {
      if (event.target instanceof Node && !rootRef.current?.contains(event.target)) {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", closeOutside);
    return () => document.removeEventListener("pointerdown", closeOutside);
  }, [open]);

  function selectAsset(asset: ArkAsset) {
    if (asset.status.toLowerCase() !== "active" || selectedIds.has(asset.id)) {
      return;
    }
    setImages((current) => {
      if (current.length >= maxImages || current.some((image) => image.assetId === asset.id)) {
        return current;
      }
      return [...current, assetToReferenceImage(asset)];
    });
  }

  async function importAsset() {
    setImporting(true);
    setError(null);
    try {
      const asset = await createArkAsset({ name: name.trim(), sourceUrl: sourceUrl.trim() });
      setAssets((current) => [asset, ...current.filter((item) => item.id !== asset.id)]);
      setName("");
      setSourceUrl("");
      if (asset.status.toLowerCase() === "active") {
        selectAsset(asset);
      }
    } catch (nextError) {
      setError(String(nextError));
    } finally {
      setImporting(false);
    }
  }

  return (
    <div className="seedance-assets" ref={rootRef}>
      <button
        aria-expanded={open}
        className="seedance-assets-trigger secondary-button"
        onClick={() => setOpen((current) => !current)}
        type="button"
      >
        <Archive size={15} />
        Ark assets
        <ChevronDown className={open ? "rotate-180" : ""} size={14} />
      </button>
      {open ? (
        <div className="seedance-assets-popover">
          <div className="seedance-assets-heading">
            <div>
              <strong>Seedance assets</strong>
              <span>Select an Active virtual portrait asset.</span>
            </div>
            <button
              aria-label="Refresh Ark assets"
              className="icon-button"
              disabled={loading}
              onClick={() => void refresh()}
              type="button"
            >
              <RefreshCw className={loading ? "spin" : ""} size={15} />
            </button>
          </div>
          {error ? <div className="seedance-assets-error">{error}</div> : null}
          <div className="seedance-assets-list">
            {loading && assets.length === 0 ? (
              <div className="seedance-assets-empty"><Loader2 className="spin" size={16} /> Loading assets…</div>
            ) : assets.length === 0 ? (
              <div className="seedance-assets-empty">No Ark image assets found.</div>
            ) : assets.map((asset) => {
              const active = asset.status.toLowerCase() === "active";
              const selected = selectedIds.has(asset.id);
              return (
                <button
                  className="seedance-asset-row"
                  disabled={!active || selected || images.length >= maxImages}
                  key={asset.id}
                  onClick={() => selectAsset(asset)}
                  title={active ? asset.id : `Asset status: ${asset.status}`}
                  type="button"
                >
                  <Archive aria-hidden="true" size={17} />
                  <span>
                    <strong>{asset.name}</strong>
                    <code>{asset.id}</code>
                  </span>
                  {selected ? <Check aria-label="Selected" size={15} /> : (
                    <small className={active ? "active" : ""}>{asset.status}</small>
                  )}
                </button>
              );
            })}
          </div>
          <div className="seedance-assets-import">
            <strong><Link2 size={14} /> Import from public URL</strong>
            <input
              aria-label="Ark asset name"
              maxLength={64}
              onChange={(event) => setName(event.target.value)}
              placeholder="Asset name"
              value={name}
            />
            <input
              aria-label="Public image URL"
              onChange={(event) => setSourceUrl(event.target.value)}
              placeholder="https://…/portrait.png"
              type="url"
              value={sourceUrl}
            />
            <button
              className="secondary-button"
              disabled={importing || !name.trim() || !sourceUrl.trim()}
              onClick={() => void importAsset()}
              type="button"
            >
              {importing ? <Loader2 className="spin" size={14} /> : <Link2 size={14} />}
              Import and get asset ID
            </button>
            <p>Ark fetches this URL. Local files require uploading to your own public storage first.</p>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function assetToReferenceImage(asset: ArkAsset): ReferenceImageInput {
  return {
    id: `ark-${asset.id}`,
    name: asset.name,
    mimeType: "image/ark-asset",
    data: "",
    dataUrl: "",
    assetId: asset.id,
  };
}
