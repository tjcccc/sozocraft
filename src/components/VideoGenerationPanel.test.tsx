import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { getVideoModelConfig, getVideoProviderConfig } from "../models/videoProviders";
import type { VideoProviderId } from "../types";
import { ModeSwitch } from "./common";
import { VideoGenerationPanel } from "./VideoGenerationPanel";

afterEach(cleanup);

function renderPanel(
  provider: VideoProviderId = "grok-imagine",
  overrides: Partial<Parameters<typeof VideoGenerationPanel>[0]> = {},
) {
  const config = getVideoProviderConfig(provider);
  const model = config.models[0];
  const props: Parameters<typeof VideoGenerationPanel>[0] = {
    allowedDurations: model.durations,
    aspectRatio: config.defaultAspectRatio,
    duration: model.defaultDuration,
    generateAudio: provider === "seedance",
    inputImages: [],
    inputImageRoles: {},
    model: model.id,
    modelConfig: model,
    onPreviewImages: vi.fn(),
    provider,
    providerConfig: config,
    resolution: model.defaultResolution,
    setAspectRatio: vi.fn(),
    setDuration: vi.fn(),
    setGenerateAudio: vi.fn(),
    setInputImages: vi.fn(),
    setInputImageRole: vi.fn(),
    setModel: vi.fn(),
    setProvider: vi.fn(),
    setResolution: vi.fn(),
    ...overrides,
  };
  render(<VideoGenerationPanel {...props} />);
  return props;
}

describe("video generation controls", () => {
  it("orders providers as Seedance, Grok Imagine, and Google Veo", async () => {
    const user = userEvent.setup();
    const setMode = vi.fn();
    render(<ModeSwitch mode="image" setMode={setMode} />);
    const props = renderPanel();

    await user.click(screen.getByRole("button", { name: "Video" }));
    expect(setMode).toHaveBeenCalledWith("video");
    expect(
      screen.getAllByRole("button").filter((button) =>
        ["Seedance", "Grok Imagine", "Google Veo"].includes(button.getAttribute("aria-label") ?? ""),
      ).map((button) => button.getAttribute("aria-label")),
    ).toEqual(["Seedance", "Grok Imagine", "Google Veo"]);

    await user.click(screen.getByRole("button", { name: "Google Veo" }));
    expect(props.setProvider).toHaveBeenCalledWith("google-veo");
  });

  it("exposes the active provider's documented controls", async () => {
    const user = userEvent.setup();
    const props = renderPanel("seedance");

    expect(screen.getByRole("option", { name: "Seedance 2.0" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "Seedance 2.0 Fast" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "Seedance 2.0 Mini" })).toBeTruthy();
    expect(screen.getByText("0 / 9")).toBeTruthy();
    expect(screen.getByText("Generate audio")).toBeTruthy();
    fireEvent.change(screen.getByRole("slider", { name: "Duration" }), {
      target: { value: "15" },
    });
    await user.selectOptions(screen.getByLabelText("Aspect Ratio"), "21:9");
    await user.selectOptions(screen.getByLabelText("Resolution"), "1080p");
    expect(props.setDuration).toHaveBeenCalledWith(15);
    expect(props.setAspectRatio).toHaveBeenCalledWith("21:9");
    expect(props.setResolution).toHaveBeenCalledWith("1080p");
  });

  it("switches Seedance models and exposes only their supported resolutions", async () => {
    const user = userEvent.setup();
    const setModel = vi.fn();
    const mini = getVideoModelConfig("seedance", "doubao-seedance-2-0-mini-260615");
    const props = renderPanel("seedance", {
      model: mini.id,
      modelConfig: mini,
      resolution: "720p",
      setModel,
    });

    expect(screen.queryByRole("option", { name: "1080p" })).toBeNull();
    await user.selectOptions(screen.getByLabelText("Model"), "doubao-seedance-2-0-fast-260128");
    expect(props.setModel).toHaveBeenCalledWith("doubao-seedance-2-0-fast-260128");
  });

  it("uses per-image roles and enforces Veo's reference duration", async () => {
    const user = userEvent.setup();
    const setInputImageRole = vi.fn();
    renderPanel("google-veo", {
      allowedDurations: [8],
      duration: 8,
      inputImages: [{
        id: "reference-image",
        name: "reference.png",
        mimeType: "image/png",
        data: "iVBORw0KGgo=",
        dataUrl: "data:image/png;base64,iVBORw0KGgo=",
      }],
      inputImageRoles: { "reference-image": "reference" },
      setInputImageRole,
    });

    expect(screen.getByText("1 / 3")).toBeTruthy();
    const slider = screen.getByRole("slider", { name: "Duration" }) as HTMLInputElement;
    expect(slider.min).toBe("8");
    expect(slider.max).toBe("8");
    expect(slider.disabled).toBe(true);
    await user.click(screen.getByRole("button", { name: "Set use for reference.png" }));
    await user.click(screen.getByRole("menuitemradio", { name: "Start frame" }));
    expect(setInputImageRole).toHaveBeenCalledWith("reference-image", "starting");
  });

  it("shows labels only for assigned start and end frames", async () => {
    const user = userEvent.setup();
    renderPanel("seedance", {
      inputImages: [
        {
          id: "start-image",
          name: "start.png",
          mimeType: "image/png",
          data: "c3RhcnQ=",
          dataUrl: "data:image/png;base64,c3RhcnQ=",
        },
        {
          id: "end-image",
          name: "end.png",
          mimeType: "image/png",
          data: "ZW5k",
          dataUrl: "data:image/png;base64,ZW5k",
        },
      ],
      inputImageRoles: { "start-image": "starting", "end-image": "ending" },
    });

    expect(screen.getByText("Start")).toBeTruthy();
    expect(screen.getByText("End")).toBeTruthy();
    expect(screen.queryByText("Reference")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Set use for start.png" }));
    expect(screen.getByRole("menuitemradio", { name: "Reference" })).toBeTruthy();
    expect(screen.getByRole("menuitemradio", { name: "End frame" })).toBeTruthy();
    await user.click(screen.getByText("Input Images"));
    expect(screen.queryByRole("menuitemradio", { name: "Reference" })).toBeNull();
  });
});
