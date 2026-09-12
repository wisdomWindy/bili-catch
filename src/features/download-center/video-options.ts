import type { MediaOption, VideoCodec, VideoVariant } from "./contracts";

const codecOrder: readonly VideoCodec[] = ["avc", "hevc", "av1"];

export function availableCodecsForQuality(
  variants: VideoVariant[],
  qualityId: string | null,
): VideoCodec[] {
  if (qualityId === null) return [];
  return codecOrder.filter((codec) =>
    variants.some((variant) => variant.qualityId === qualityId && variant.codec === codec),
  );
}

export function isVideoVariantValid(
  variants: VideoVariant[],
  qualityId: string | null,
  codec: VideoCodec | null,
): boolean {
  return qualityId !== null && codec !== null && variants.some(
    (variant) => variant.qualityId === qualityId && variant.codec === codec,
  );
}

export function selectVideoVariant(
  qualities: MediaOption[],
  variants: VideoVariant[],
  preferredQualityId: string | null,
  preferredCodec: VideoCodec | null,
): VideoVariant | null {
  const availableQualities = qualities.filter((quality) => !quality.requiresLogin);
  const qualityId = availableQualities.some(
    (quality) => quality.id === preferredQualityId && availableCodecsForQuality(variants, quality.id).length > 0,
  )
    ? preferredQualityId
    : availableQualities.find(
      (quality) => availableCodecsForQuality(variants, quality.id).length > 0,
    )?.id ?? null;
  const codecs = availableCodecsForQuality(variants, qualityId);
  const codec = preferredCodec !== null && codecs.includes(preferredCodec)
    ? preferredCodec
    : codecs[0] ?? null;
  return qualityId !== null && codec !== null ? { qualityId, codec } : null;
}
