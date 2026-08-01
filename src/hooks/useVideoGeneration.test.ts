import { describe, expect, it } from "vitest";
import {
  getVideoDurations,
  getVideoModelConfig,
  getVideoProviderConfig,
} from "../models/videoProviders";
import {
  buildVideoGenerationRequest,
  reconcileInputImageRoles,
  updateInputImageRole,
  videoInputMode,
} from "./useVideoGeneration";

const startingImage = {
  id: "starting-image-id",
  name: "starting.png",
  mimeType: "image/png",
  data: "iVBORw0KGgo=",
  dataUrl: "data:image/png;base64,iVBORw0KGgo=",
};

describe("video provider request mapping", () => {
  it("derives input mode without a separate mode selector", () => {
    const endingImage = { ...startingImage, id: "ending-image-id", name: "ending.png" };
    expect(videoInputMode([], {})).toBe("text");
    expect(videoInputMode([startingImage], { [startingImage.id]: "starting" })).toBe("image");
    expect(videoInputMode([startingImage], { [startingImage.id]: "reference" })).toBe("reference");
    expect(videoInputMode(
      [startingImage, endingImage],
      { [startingImage.id]: "starting", [endingImage.id]: "ending" },
    )).toBe("frames");
  });

  it("keeps provider image roles in valid workflows", () => {
    const endingImage = { ...startingImage, id: "ending-image-id", name: "ending.png" };
    const paired = reconcileInputImageRoles(
      "google-veo",
      [startingImage],
      [startingImage, endingImage],
      { [startingImage.id]: "starting" },
    );
    expect(paired).toEqual({
      [startingImage.id]: "starting",
      [endingImage.id]: "ending",
    });

    const swapped = updateInputImageRole(
      "seedance",
      [startingImage, endingImage],
      {
        [startingImage.id]: "starting",
        [endingImage.id]: "ending",
      },
      startingImage.id,
      "ending",
    );
    expect(swapped).toEqual({
      [startingImage.id]: "ending",
      [endingImage.id]: "starting",
    });

    expect(updateInputImageRole(
      "grok-imagine",
      [startingImage, endingImage],
      { [startingImage.id]: "reference", [endingImage.id]: "reference" },
      startingImage.id,
      "starting",
    )).toEqual({ [startingImage.id]: "reference", [endingImage.id]: "reference" });
  });

  it("encodes the documented provider capabilities", () => {
    expect(getVideoProviderConfig("seedance").maxInputImages).toBe(9);
    expect(getVideoProviderConfig("grok-imagine").maxInputImages).toBe(7);
    expect(getVideoProviderConfig("google-veo").maxInputImages).toBe(3);
    expect(getVideoDurations("grok-imagine", "grok-imagine-video", "reference", "720p")).toEqual([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    ]);
    expect(getVideoDurations("google-veo", "veo-3.1-generate-preview", "text", "720p"))
      .toEqual([4, 6, 8]);
    expect(getVideoDurations("google-veo", "veo-3.1-generate-preview", "reference", "720p"))
      .toEqual([8]);
    expect(getVideoDurations("google-veo", "veo-3.1-generate-preview", "text", "4k"))
      .toEqual([8]);
    expect(getVideoModelConfig("seedance", "doubao-seedance-2-0-fast-260128").resolutions)
      .toEqual(["480p", "720p"]);
    expect(getVideoModelConfig("seedance", "doubao-seedance-2-0-mini-260615").durations[0])
      .toBe(4);
  });

  it("maps a Google Veo starting image through the shared input behavior", () => {
    const request = buildVideoGenerationRequest({
      aspectRatio: "9:16",
      duration: 8,
      generateAudio: true,
      inputMode: "image",
      model: "veo-3.1-generate-preview",
      prompt: "Animate the water",
      promptSnapshot: "Animate the water",
      provider: "google-veo",
      referenceImages: [],
      resolution: "1080p",
      startingImage,
      taskId: "cf5da7bc-36b9-4ac8-a4e8-d6464cd3da09",
    });

    expect(request.provider).toBe("google-veo");
    expect(request.model).toBe("veo-3.1-generate-preview");
    expect(request.startingImage).toEqual({
      name: "starting.png",
      mimeType: "image/png",
      data: "iVBORw0KGgo=",
    });
    expect(request.options).toEqual({
      duration: 8,
      aspectRatio: "9:16",
      resolution: "1080p",
    });
  });

  it("maps Seedance references and its audio option", () => {
    const request = buildVideoGenerationRequest({
      aspectRatio: "21:9",
      duration: 15,
      generateAudio: false,
      inputMode: "reference",
      model: "doubao-seedance-2-0-260128",
      prompt: "Keep the subjects consistent",
      promptSnapshot: "Keep the subjects consistent",
      provider: "seedance",
      referenceImages: [
        startingImage,
        { ...startingImage, id: "second-id", name: "second.png" },
      ],
      resolution: "1080p",
      taskId: "cf5da7bc-36b9-4ac8-a4e8-d6464cd3da09",
    });

    expect(request.startingImage).toBeUndefined();
    expect(request.referenceImages).toHaveLength(2);
    expect(request.options.generateAudio).toBe(false);
  });

  it("maps a selected Ark asset into a Seedance reference payload", () => {
    const asset = {
      id: "ark-asset-20260801-example",
      name: "portrait",
      mimeType: "image/ark-asset",
      data: "",
      dataUrl: "",
      assetId: "asset-20260801-example",
    };
    const request = buildVideoGenerationRequest({
      aspectRatio: "16:9",
      duration: 10,
      generateAudio: true,
      inputMode: "reference",
      model: "doubao-seedance-2-0-260128",
      prompt: "Keep the virtual portrait identity consistent",
      promptSnapshot: "Keep the virtual portrait identity consistent",
      provider: "seedance",
      referenceImages: [asset],
      resolution: "720p",
      taskId: "cf5da7bc-36b9-4ac8-a4e8-d6464cd3da09",
    });

    expect(request.referenceImages).toEqual([{
      name: "portrait",
      mimeType: "image/ark-asset",
      data: "",
      assetId: "asset-20260801-example",
    }]);
  });

  it("keeps Ark assets in reference mode when mixed with local images", () => {
    const asset = {
      ...startingImage,
      id: "ark-asset-20260801-example",
      data: "",
      dataUrl: "",
      assetId: "asset-20260801-example",
    };

    expect(reconcileInputImageRoles(
      "seedance",
      [startingImage],
      [startingImage, asset],
      { [startingImage.id]: "starting" },
    )).toEqual({
      [startingImage.id]: "reference",
      [asset.id]: "reference",
    });
  });

  it("maps Veo start and end frames", () => {
    const endingImage = { ...startingImage, id: "ending-id", name: "ending.png" };
    const request = buildVideoGenerationRequest({
      aspectRatio: "16:9",
      duration: 8,
      endingImage,
      generateAudio: true,
      inputMode: "frames",
      model: "veo-3.1-generate-preview",
      prompt: "Move from the first frame to the last frame",
      promptSnapshot: "Move from the first frame to the last frame",
      provider: "google-veo",
      referenceImages: [],
      resolution: "720p",
      startingImage,
      taskId: "cf5da7bc-36b9-4ac8-a4e8-d6464cd3da09",
    });

    expect(request.inputMode).toBe("frames");
    expect(request.startingImage?.name).toBe("starting.png");
    expect(request.endingImage?.name).toBe("ending.png");
    expect(request.referenceImages).toBeUndefined();
  });
});
