import { describe, expect, it } from "vitest";
import { getProviderModels, normalizeProviderOptions } from "./imageProviders";

const options = { aspectRatio: "auto", imageSize: "auto", quality: "max", thinkingLevel: "" };

describe("image model platform catalogs", () => {
  it("offers both GPT 2.5 variants and retains GPT 2 on OpenAI and Higgsfield", () => {
    expect(getProviderModels("gpt-image", "openai").map((model) => model.id)).toEqual([
      "gpt-image-2.5-flare", "gpt-image-2.5-sunburst", "gpt-image-2",
    ]);
    expect(getProviderModels("gpt-image", "higgsfield").map((model) => model.id)).toEqual([
      "gpt_image_2_5_flare", "gpt_image_2_5_sunburst", "gpt_image_2",
    ]);
    expect(getProviderModels("gpt-image", "openrouter").map((model) => model.id)).toEqual([
      "openai/gpt-image-2.5-flare", "openai/gpt-image-2.5-sunburst", "openai/gpt-image-2",
    ]);
  });

  it("keeps GPT 2.5 quality and clamps it when returning to GPT 2", () => {
    for (const model of ["gpt-image-2.5-flare", "gpt-image-2.5-sunburst"]) {
      expect(normalizeProviderOptions("gpt-image", model, options, "openai")).toMatchObject({ model, quality: "max" });
    }
    expect(normalizeProviderOptions("gpt-image", "gpt-image-2", options, "openai").quality).toBe("auto");
    expect(normalizeProviderOptions("gpt-image", "gpt_image_2_5_sunburst", options, "higgsfield"))
      .toMatchObject({ model: "gpt_image_2_5_sunburst", quality: "max", imageSize: "1k" });
  });

  it("uses documented OpenRouter image controls for both 2.5 variants", () => {
    for (const model of ["openai/gpt-image-2.5-flare", "openai/gpt-image-2.5-sunburst"]) {
      expect(normalizeProviderOptions("gpt-image", model, { ...options, aspectRatio: "16:9" }, "openrouter"))
        .toMatchObject({ model, quality: "max", aspectRatio: "16:9", imageSize: "" });
    }
  });

  it("migrates the OpenRouter chat model to direct GPT Image 2", () => {
    expect(normalizeProviderOptions("gpt-image", "openai/gpt-5.4-image-2", options, "openrouter"))
      .toMatchObject({ model: "openai/gpt-image-2", quality: "auto", imageSize: "" });
  });

  it("migrates old xAI models and quality to Grok 2.0 while retaining Higgsfield", () => {
    expect(getProviderModels("grok-imagine", "xai").map((model) => model.id)).toEqual(["grok-imagine-image-2.0"]);
    for (const model of ["grok-imagine-image", "grok-imagine-image-quality"]) {
      expect(normalizeProviderOptions("grok-imagine", model, { ...options, quality: "high" }, "xai"))
        .toMatchObject({ model: "grok-imagine-image-2.0", quality: "auto", imageSize: "1k" });
    }
    expect(getProviderModels("grok-imagine", "higgsfield").map((model) => model.id)).toEqual(["grok_image"]);
  });
});
