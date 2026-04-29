export type GenerationStatus = "queued" | "running" | "completed" | "failed";

export type ConfigStatus = {
  configPath: string;
  hasApiKey: boolean;
  hasOpenaiApiKey: boolean;
  hasXaiApiKey: boolean;
  hasProxy: boolean;
};

export type AppSettings = {
  defaultProvider: "nano-banana" | "gpt-image" | "grok-imagine";
  defaultModel: string;
  outputDirectory: string;
  outputTemplate: string;
  optionalBaseUrl?: string | null;
  openaiBaseUrl?: string | null;
  xaiBaseUrl?: string | null;
  proxyUrl?: string | null;
  timeoutSeconds: number;
};

export type AppState = {
  settings: AppSettings;
  currentPrompt: string;
  batches: GenerationBatch[];
};

export type GenerationOptions = {
  aspectRatio?: string | null;
  imageSize?: string | null;
  temperature?: number | null;
  topP?: number | null;
  thinkingLevel?: string | null;
  quality?: string | null;
};

export type GenerationRequest = {
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

export type ReferenceImageInput = {
  id: string;
  name: string;
  mimeType: string;
  data: string;
  dataUrl: string;
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

export type GenerationBatch = {
  id: string;
  provider: string;
  model: string;
  promptSnapshot: string;
  status: GenerationStatus;
  images: OutputImage[];
  createdAt: string;
  completedAt?: string | null;
  error?: string | null;
};
