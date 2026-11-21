import { describe, expect, it } from "vitest";
import type { AppSettings } from "../types";
import {
  getProviderControlConfig,
  getProviderModels,
  normalizeProviderOptions,
  settingsPlatformForProvider,
} from "./imageProviders";

const options = { aspectRatio: "auto", imageSize: "auto", quality: "max", thinkingLevel: "" };

describe("image model platform catalogs", () => {
  it("offers both GPT Image versions on OpenAI and OpenRouter, and only 2.5 on Higgsfield", () => {
    expect(getProviderModels("gpt-image", "openai").map((model) => model.id)).toEqual([
      "gpt-image-2.6-flare", "gpt-image-2.6-sunburst", "gpt-image-2.5-flare", "gpt-image-2.5-sunburst",
    ]);
    expect(getProviderModels("gpt-image", "openrouter").map((model) => model.id)).toEqual([
      "openai/gpt-image-2.6-flare", "openai/gpt-image-2.6-sunburst",
      "openai/gpt-image-2.5-flare", "openai/gpt-image-2.5-sunburst",
    ]);
    expect(getProviderModels("gpt-image", "higgsfield").map((model) => model.id)).toEqual([
      "gpt_image_2_5_flare", "gpt_image_2_5_sunburst",
    ]);
  });

  it("keeps extended quality for both GPT Image versions across their platforms", () => {
    for (const platform of ["openai", "openrouter", "higgsfield"] as const) {
      for (const { id: model } of getProviderModels("gpt-image", platform)) {
        expect(normalizeProviderOptions("gpt-image", model, options, platform))
          .toMatchObject({ model, quality: "max" });
      }
    }
    expect(normalizeProviderOptions("gpt-image", "gpt_image_2_5_sunburst", options, "higgsfield"))
      .toMatchObject({ imageSize: "1k" });
  });

  it("uses aspect ratios for OpenRouter GPT Image models", () => {
    for (const { id: model } of getProviderModels("gpt-image", "openrouter")) {
      expect(normalizeProviderOptions("gpt-image", model, { ...options, aspectRatio: "16:9" }, "openrouter"))
        .toMatchObject({ model, aspectRatio: "16:9", imageSize: "" });
    }
  });

  it("offers Nano Banana 2.1 on Gemini with medium thinking and no 512 size", () => {
    expect(getProviderModels("nano-banana", "gemini").map((model) => model.id)).toEqual([
      "gemini-nano-banana-2.1", "gemini-3.1-flash-image-preview", "gemini-3-pro-image-preview", "gemini-2.5-flash-image",
    ]);
    expect(normalizeProviderOptions("nano-banana", "gemini-nano-banana-2.1", {
      ...options, aspectRatio: "8:1", imageSize: "512", thinkingLevel: "medium",
    }, "gemini")).toMatchObject({ aspectRatio: "8:1", imageSize: "2K", thinkingLevel: "medium" });
    expect(normalizeProviderOptions("nano-banana", "gemini-3.1-flash-image-preview", {
      ...options, thinkingLevel: "medium",
    }, "gemini").thinkingLevel).toBe("minimal");
    expect(normalizeProviderOptions("nano-banana", "gemini-nano-banana-2.1", options, "gemini").thinkingLevel)
      .toBe("medium");
  });

  it("migrates legacy selections to an available Flare model on the configured platform", () => {
    expect(normalizeProviderOptions("gpt-image", "gpt-image-2", options, "openai").model)
      .toBe("gpt-image-2.6-flare");
    for (const model of ["openai/gpt-image-2", "openai/gpt-5.4-image-2"]) {
      expect(normalizeProviderOptions("gpt-image", model, options, "openrouter"))
        .toMatchObject({ model: "openai/gpt-image-2.6-flare", imageSize: "" });
    }
    for (const model of ["gpt_image_2", "gpt-image-2.6-sunburst"]) {
      expect(normalizeProviderOptions("gpt-image", model, options, "higgsfield"))
        .toMatchObject({ model: "gpt_image_2_5_flare", imageSize: "1k" });
    }
  });

  it("migrates old xAI models and quality to Grok 2.0 while retaining Higgsfield", () => {
    expect(getProviderModels("grok-imagine", "xai").map((model) => model.id)).toEqual(["grok-imagine-image-2.0"]);
    for (const model of ["grok-imagine-image", "grok-imagine-image-quality"]) {
      expect(normalizeProviderOptions("grok-imagine", model, { ...options, quality: "high" }, "xai"))
        .toMatchObject({ model: "grok-imagine-image-2.0", quality: "auto", imageSize: "1k" });
    }
    expect(getProviderModels("grok-imagine", "higgsfield").map((model) => model.id)).toEqual(["grok_image"]);
  });

  it("offers Muse Image in Experimental with its own controls and OpenRouter routing", () => {
    expect(getProviderModels("experimental", "openrouter").map((model) => model.id)).toEqual([
      "meta/muse-image",
      "bytedance-seed/seedream-5-0-pro",
      "bytedance-seed/seedream-5-0-flash",
      "bytedance-seed/seedream-5-0-lite",
    ]);
    expect(normalizeProviderOptions(
      "experimental",
      "bytedance-seed/seedream-5-0-lite",
      { ...options, aspectRatio: "9:19.5", imageSize: "1K" },
      "openrouter",
    )).toMatchObject({ aspectRatio: "9:19.5", imageSize: "2K", maxReferenceImages: 14 });
    const controls = getProviderControlConfig("experimental", "meta/muse-image", "openrouter");
    expect(controls.aspectRatios).toContain("9:21");
    expect(controls.imageSizes).toBeNull();
    expect(controls.qualityLevels).toBeNull();
    expect(controls.maxReferenceImages).toBe(10);
    expect(normalizeProviderOptions("experimental", "meta/muse-image", options, "openrouter"))
      .toMatchObject({ model: "meta/muse-image", aspectRatio: "auto", imageSize: "", quality: "" });
    expect(settingsPlatformForProvider(
      { grokApiPlatform: "higgsfield", openaiApiPlatform: "openai" } as AppSettings,
      "experimental",
    )).toBe("openrouter");
  });
});
