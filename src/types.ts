export type GenerationStatus = "queued" | "running" | "completed" | "failed" | "cancelled";
export type GenerationMediaType = "image" | "video";
export type PromptPreviewPlacement = "bottom" | "right" | "hidden";
export type NanoBananaApiPlatform = "gemini" | "higgsfield";
export type OpenAiApiPlatform = "openai" | "openrouter" | "higgsfield";
export type GrokImagineApiPlatform = "xai" | "higgsfield";
export type SeedanceApiPlatform = "ark" | "higgsfield";

export type ConfigStatus = {
  configPath: string;
  hasApiKey: boolean;
  hasOpenaiApiKey: boolean;
  hasOpenrouterApiKey: boolean;
  hasXaiApiKey: boolean;
  hasArkApiKey: boolean;
  hasArkAssetCredentials: boolean;
  hasProxy: boolean;
};

export type HiggsfieldStatus = {
  cliPath: string;
  installed: boolean;
  authenticated: boolean;
  version?: string | null;
  account?: string | null;
  error?: string | null;
};

export type AppSettings = {
  defaultProvider: "nano-banana" | "gpt-image" | "grok-imagine";
  defaultModel: string;
  outputDirectory: string;
  outputTemplate: string;
  promptDirectory: string;
  promptDslEnabled: boolean;
  promptEditorOnly: boolean;
  promptPreviewPlacement: PromptPreviewPlacement;
  nanoBananaApiPlatform: NanoBananaApiPlatform;
  geminiProxyEnabled: boolean;
  openaiApiPlatform: OpenAiApiPlatform;
  openaiProxyEnabled: boolean;
  grokApiPlatform: GrokImagineApiPlatform;
  xaiProxyEnabled: boolean;
  seedanceApiPlatform: SeedanceApiPlatform;
  seedanceDefaultModel: string;
  arkProxyEnabled: boolean;
  higgsfieldCliPath?: string | null;
  optionalBaseUrl?: string | null;
  openaiBaseUrl?: string | null;
  openrouterBaseUrl?: string | null;
  xaiBaseUrl?: string | null;
  arkBaseUrl?: string | null;
  proxyUrl?: string | null;
  timeoutSeconds: number;
  geminiTimeoutSeconds: number;
  openaiTimeoutSeconds: number;
  xaiTimeoutSeconds: number;
  arkTimeoutSeconds: number;
};

export type AppState = {
  settings: AppSettings;
  currentPrompt: string;
  currentPromptId?: string | null;
  batches: GenerationBatch[];
};

export type GenerationOptions = {
  aspectRatio?: string | null;
  imageSize?: string | null;
  temperature?: number | null;
  topP?: number | null;
  thinkingLevel?: string | null;
  quality?: string | null;
  unlimited?: boolean | null;
};

export type ImageTextMetadata = Record<string, string>;

export type GenerationRequest = {
  taskId?: string | null;
  provider: "nano-banana" | "gpt-image" | "grok-imagine";
  model: string;
  prompt: string;
  promptSnapshot?: string | null;
  batchCount: number;
  referenceImages?: ReferenceImageInput[];
  outputTemplate: string;
  options: GenerationOptions;
  baseUrl?: string | null;
};

export type VideoGenerationOptions = {
  duration: number;
  aspectRatio: string;
  resolution: string;
  generateAudio?: boolean | null;
};

export type VideoInputMode = "text" | "image" | "frames" | "reference";
export type VideoInputImageRole = "reference" | "starting" | "ending";
export type VideoProviderId = "seedance" | "grok-imagine" | "google-veo";

export type VideoGenerationRequest = {
  taskId?: string | null;
  provider: VideoProviderId;
  model: string;
  prompt: string;
  promptSnapshot?: string | null;
  inputMode: VideoInputMode;
  startingImage?: ReferenceImagePayload | null;
  endingImage?: ReferenceImagePayload | null;
  referenceImages?: ReferenceImagePayload[] | null;
  options: VideoGenerationOptions;
};

export type ReferenceImageInput = {
  id: string;
  name: string;
  mimeType: string;
  data: string;
  dataUrl: string;
  assetId?: string;
};

export type ReferenceImagePayload = Pick<ReferenceImageInput, "name" | "mimeType" | "data"> & {
  assetId?: string;
};

export type ArkAsset = {
  id: string;
  name: string;
  status: string;
  groupId: string;
  assetType: string;
  createTime?: string | null;
};

export type CreateArkAssetRequest = {
  name: string;
  sourceUrl: string;
};

export type OutputImage = {
  id: string;
  batchId: string;
  provider: string;
  model: string;
  path: string;
  filename: string;
  createdAt: string;
  promptSnapshot: string;
  metadata?: Record<string, unknown> | null;
};

export type OutputVideo = {
  id: string;
  batchId: string;
  provider: string;
  model: string;
  path: string;
  filename: string;
  metadataPath: string;
  createdAt: string;
  promptSnapshot: string;
  duration: number;
  aspectRatio: string;
  resolution: string;
  metadata?: Record<string, unknown> | null;
};

export type GenerationBatch = {
  id: string;
  mediaType: GenerationMediaType;
  provider: string;
  model: string;
  promptSnapshot: string;
  status: GenerationStatus;
  images: OutputImage[];
  videos: OutputVideo[];
  providerRequestId?: string | null;
  createdAt: string;
  completedAt?: string | null;
  error?: string | null;
};

export type PromptListItem = {
  id: string;
  path: string;
  name: string;
  tags: string[];
  description: string;
  createdAt?: string | null;
  updatedAt?: string | null;
};

export type PromptDocument = {
  item: PromptListItem;
  source: string;
  renderedPrompt: string;
};

export type CreatePromptRequest = {
  name: string;
  tags?: string[];
  description?: string;
};

export type SavePromptRequest = {
  id: string;
  source: string;
};

export type UpdatePromptMetadataRequest = {
  id: string;
  name: string;
  tags: string[];
};

export type RenamePromptTagRequest = {
  oldTagPath: string;
  newTagPath: string;
};

export type RenderPromptResult = {
  renderedPrompt: string;
};
