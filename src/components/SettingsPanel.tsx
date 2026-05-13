import { KeyRound, RefreshCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { checkHiggsfieldStatus } from "../api";
import type {
  AppSettings,
  ConfigStatus,
  GrokImagineApiPlatform,
  HiggsfieldStatus,
  NanoBananaApiPlatform,
  OpenAiApiPlatform,
  PromptPreviewPlacement,
} from "../types";
import {
  GPT_IMAGE_PLATFORM_MODELS,
  GROK_IMAGE_PLATFORM_MODELS,
  IMAGE_PROVIDER_IDS,
  NANO_BANANA_PLATFORM_MODELS,
  getProviderConfig,
  getProviderModels,
} from "../models/imageProviders";
import type { ImageProviderApiPlatform } from "../models/imageProviders";
import { validateOutputTemplate } from "../utils/outputTemplate";
import { Field, ToggleSwitch } from "./common";

export function SettingsPanel({
  apiKey,
  apiKeySaved,
  openaiApiKey,
  openaiApiKeySaved,
  openrouterApiKey,
  openrouterApiKeySaved,
  xaiApiKey,
  xaiApiKeySaved,
  configStatus,
  settings,
  setApiKey,
  setOpenaiApiKey,
  setOpenrouterApiKey,
  setSettings,
  setXaiApiKey,
  onSaveKey,
  onSaveOpenaiKey,
  onSaveOpenrouterKey,
  onSaveSettings,
  onSaveXaiKey,
}: {
  apiKey: string;
  apiKeySaved: boolean;
  openaiApiKey: string;
  openaiApiKeySaved: boolean;
  openrouterApiKey: string;
  openrouterApiKeySaved: boolean;
  xaiApiKey: string;
  xaiApiKeySaved: boolean;
  configStatus: ConfigStatus | null;
  settings: AppSettings;
  setApiKey: (value: string) => void;
  setOpenaiApiKey: (value: string) => void;
  setOpenrouterApiKey: (value: string) => void;
  setSettings: (settings: AppSettings) => void;
  setXaiApiKey: (value: string) => void;
  onSaveKey: () => void;
  onSaveOpenaiKey: () => void;
  onSaveOpenrouterKey: () => void;
  onSaveSettings: (settings: AppSettings) => void;
  onSaveXaiKey: () => void;
}) {
  const templateIssues = validateOutputTemplate(settings.outputTemplate);
  const didMountRef = useRef(false);
  const [higgsfieldStatus, setHiggsfieldStatus] = useState<HiggsfieldStatus | null>(null);
  const [checkingHiggsfield, setCheckingHiggsfield] = useState(false);
  const higgsfieldPlan = planNameFromAccount(higgsfieldStatus?.account);

  useEffect(() => {
    if (!didMountRef.current) {
      didMountRef.current = true;
      return;
    }
    const handle = window.setTimeout(() => onSaveSettings(settings), 650);
    return () => window.clearTimeout(handle);
  }, [onSaveSettings, settings]);

  async function verifyHiggsfield() {
    setCheckingHiggsfield(true);
    try {
      setHiggsfieldStatus(
        await checkHiggsfieldStatus(settings.higgsfieldCliPath, settings.proxyUrl),
      );
    } finally {
      setCheckingHiggsfield(false);
    }
  }

  return (
    <section className="settings-panel">
      <aside className="settings-sidebar" aria-label="Settings sections">
        <a href="#settings-general">General</a>
        <a href="#settings-prompts">Prompts</a>
        <a href="#settings-providers">Providers</a>
      </aside>

      <div className="settings-content">
        {configStatus ? (
          <div className="config-status">
            <div className="config-status-row field-full">
              <span className="config-status-label">Config file</span>
              <code className="config-status-value">
                {configStatus.configPath}
              </code>
            </div>
            <ConfigBadge
              label="Proxy"
              ok={configStatus.hasProxy}
              okText="Configured"
              missingText="Not set"
            />
            <ConfigBadge label="Gemini API key" ok={configStatus.hasApiKey} />
            <ConfigBadge
              label="OpenAI API key"
              ok={configStatus.hasOpenaiApiKey}
            />
            <ConfigBadge label="xAI API key" ok={configStatus.hasXaiApiKey} />
            <ConfigBadge
              label="OpenRouter API key"
              ok={configStatus.hasOpenrouterApiKey}
            />
          </div>
        ) : null}

        <div className="settings-section" id="settings-general">
          <h2>General</h2>
          <Field label="Output Directory">
            <input
              value={settings.outputDirectory}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  outputDirectory: event.target.value,
                })
              }
            />
          </Field>
          <Field label="Filename Template">
            <input
              value={settings.outputTemplate}
              onChange={(event) =>
                setSettings({ ...settings, outputTemplate: event.target.value })
              }
            />
            {templateIssues.length > 0 ? (
              <div className="template-issues">
                {templateIssues.map((issue, index) => (
                  <span className="template-issue" key={index}>
                    {issue}
                  </span>
                ))}
              </div>
            ) : null}
          </Field>
          <Field label="Proxy URL">
            <input
              placeholder="http://127.0.0.1:7890"
              value={settings.proxyUrl ?? ""}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  proxyUrl: event.target.value || null,
                })
              }
            />
          </Field>
          <ToggleSwitch
            checked={settings.promptEditorOnly}
            label="Prompt Editor only"
            onChange={(checked) =>
              setSettings({ ...settings, promptEditorOnly: checked })
            }
          />
        </div>

        <div className="settings-section" id="settings-prompts">
          <h2>Prompts</h2>
          <Field label="Prompt Directory">
            <input
              value={settings.promptDirectory}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  promptDirectory: event.target.value,
                })
              }
            />
          </Field>
          <Field label="Preview Placement">
            <select
              value={settings.promptPreviewPlacement}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  promptPreviewPlacement: event.target
                    .value as PromptPreviewPlacement,
                })
              }
            >
              <option value="bottom">Bottom</option>
              <option value="right">Right</option>
              <option value="hidden">Hidden</option>
            </select>
          </Field>
          <ToggleSwitch
            checked={settings.promptDslEnabled}
            label="DSL mode"
            onChange={(checked) =>
              setSettings({ ...settings, promptDslEnabled: checked })
            }
          />
        </div>

        <div className="settings-section" id="settings-providers">
          <h2>Providers</h2>
          <ProviderSettings
            apiKey={apiKey}
            apiKeySaved={apiKeySaved}
            apiPlatform={settings.nanoBananaApiPlatform}
            apiPlatformOptions={[
              { label: "Gemini", value: "gemini" },
              { label: "Higgsfield CLI", value: "higgsfield" },
            ]}
            baseUrl={settings.optionalBaseUrl}
            defaultModel={
              settings.defaultProvider === "nano-banana"
                ? settings.defaultModel
                : null
            }
            keyPlaceholder="Gemini API key"
            onApiPlatformChange={(platform) => {
              const nextPlatform = platform as NanoBananaApiPlatform;
              setSettings({
                ...settings,
                nanoBananaApiPlatform: nextPlatform,
                defaultModel:
                  settings.defaultProvider === "nano-banana"
                    ? NANO_BANANA_PLATFORM_MODELS[nextPlatform]
                    : settings.defaultModel,
              });
            }}
            onSaveKey={onSaveKey}
            providerId="nano-banana"
            proxyEnabled={settings.geminiProxyEnabled}
            setApiKey={setApiKey}
            setBaseUrl={(value) =>
              setSettings({ ...settings, optionalBaseUrl: value })
            }
            setDefaultModel={(model) =>
              setSettings({
                ...settings,
                defaultProvider: "nano-banana",
                defaultModel: model,
              })
            }
            setProxyEnabled={(value) =>
              setSettings({ ...settings, geminiProxyEnabled: value })
            }
            setTimeoutSeconds={(value) =>
              setSettings({
                ...settings,
                timeoutSeconds: value,
                geminiTimeoutSeconds: value,
              })
            }
            timeoutSeconds={settings.geminiTimeoutSeconds}
          />
          <ProviderSettings
            apiKey={
              settings.openaiApiPlatform === "openrouter"
                ? openrouterApiKey
                : openaiApiKey
            }
            apiKeySaved={
              settings.openaiApiPlatform === "openrouter"
                ? openrouterApiKeySaved
                : openaiApiKeySaved
            }
            apiPlatform={settings.openaiApiPlatform}
            apiPlatformOptions={[
              { label: "OpenAI", value: "openai" },
              { label: "OpenRouter", value: "openrouter" },
              { label: "Higgsfield CLI", value: "higgsfield" },
            ]}
            baseUrl={
              settings.openaiApiPlatform === "openrouter"
                ? settings.openrouterBaseUrl
                : settings.openaiBaseUrl
            }
            defaultModel={
              settings.defaultProvider === "gpt-image"
                ? settings.defaultModel
                : null
            }
            keyPlaceholder={
              settings.openaiApiPlatform === "openrouter"
                ? "OpenRouter API key"
                : "OpenAI API key"
            }
            onApiPlatformChange={(platform) =>
              setSettings({
                ...settings,
                openaiApiPlatform: platform as OpenAiApiPlatform,
                defaultModel:
                  settings.defaultProvider === "gpt-image"
                    ? GPT_IMAGE_PLATFORM_MODELS[platform as OpenAiApiPlatform]
                    : settings.defaultModel,
              })
            }
            onSaveKey={
              settings.openaiApiPlatform === "openrouter"
                ? onSaveOpenrouterKey
                : onSaveOpenaiKey
            }
            providerId="gpt-image"
            proxyEnabled={settings.openaiProxyEnabled}
            setApiKey={
              settings.openaiApiPlatform === "openrouter"
                ? setOpenrouterApiKey
                : setOpenaiApiKey
            }
            setBaseUrl={(value) => {
              if (settings.openaiApiPlatform === "openrouter") {
                setSettings({ ...settings, openrouterBaseUrl: value });
                return;
              }
              setSettings({ ...settings, openaiBaseUrl: value });
            }}
            setDefaultModel={(model) =>
              setSettings({
                ...settings,
                defaultProvider: "gpt-image",
                defaultModel: model,
              })
            }
            setProxyEnabled={(value) =>
              setSettings({ ...settings, openaiProxyEnabled: value })
            }
            setTimeoutSeconds={(value) =>
              setSettings({ ...settings, openaiTimeoutSeconds: value })
            }
            timeoutSeconds={settings.openaiTimeoutSeconds}
          />
          <ProviderSettings
            apiKey={xaiApiKey}
            apiKeySaved={xaiApiKeySaved}
            apiPlatform={settings.grokApiPlatform}
            apiPlatformOptions={[
              { label: "xAI", value: "xai" },
              { label: "Higgsfield CLI", value: "higgsfield" },
            ]}
            baseUrl={settings.xaiBaseUrl}
            defaultModel={
              settings.defaultProvider === "grok-imagine"
                ? settings.defaultModel
                : null
            }
            keyPlaceholder="xAI API key"
            onApiPlatformChange={(platform) => {
              const nextPlatform = platform as GrokImagineApiPlatform;
              setSettings({
                ...settings,
                grokApiPlatform: nextPlatform,
                defaultModel:
                  settings.defaultProvider === "grok-imagine"
                    ? GROK_IMAGE_PLATFORM_MODELS[nextPlatform]
                    : settings.defaultModel,
              });
            }}
            onSaveKey={onSaveXaiKey}
            providerId="grok-imagine"
            proxyEnabled={settings.xaiProxyEnabled}
            setApiKey={setXaiApiKey}
            setBaseUrl={(value) =>
              setSettings({ ...settings, xaiBaseUrl: value })
            }
            setDefaultModel={(model) =>
              setSettings({
                ...settings,
                defaultProvider: "grok-imagine",
                defaultModel: model,
              })
            }
            setProxyEnabled={(value) =>
              setSettings({ ...settings, xaiProxyEnabled: value })
            }
            setTimeoutSeconds={(value) =>
              setSettings({ ...settings, xaiTimeoutSeconds: value })
            }
            timeoutSeconds={settings.xaiTimeoutSeconds}
          />
          <fieldset className="provider-settings">
            <legend>Higgsfield CLI</legend>
            <Field label="CLI Path">
              <div className="template-row">
                <input
                  placeholder="higgsfield"
                  value={settings.higgsfieldCliPath ?? ""}
                  onChange={(event) =>
                    setSettings({
                      ...settings,
                      higgsfieldCliPath: event.target.value || null,
                    })
                  }
                />
                <button
                  className="secondary-button"
                  disabled={checkingHiggsfield}
                  onClick={() => void verifyHiggsfield()}
                  type="button"
                >
                  <RefreshCw className={checkingHiggsfield ? "spin" : undefined} size={15} />
                  Check
                </button>
              </div>
            </Field>
            {higgsfieldStatus ? (
              <div className="provider-status-grid">
                <ConfigBadge label="CLI" ok={higgsfieldStatus.installed} />
                <ConfigBadge
                  label="Auth"
                  ok={higgsfieldStatus.authenticated}
                  okText="Logged in"
                  missingText="Login needed"
                />
                <ConfigBadge
                  label="Plan"
                  ok={Boolean(higgsfieldPlan)}
                  okText={higgsfieldPlan ?? "Unknown"}
                  missingText="Unknown"
                />
                {higgsfieldStatus.version ? (
                  <div className="config-status-row field-full">
                    <span className="config-status-label">Version</span>
                    <code className="config-status-value">{higgsfieldStatus.version}</code>
                  </div>
                ) : null}
                {higgsfieldStatus.error ? (
                  <div className="config-status-row field-full">
                    <span className="config-status-label">Message</span>
                    <span className="config-status-value">{higgsfieldStatus.error}</span>
                  </div>
                ) : null}
              </div>
            ) : null}
          </fieldset>
        </div>
      </div>
    </section>
  );
}

function ProviderSettings({
  apiKey,
  apiKeySaved,
  apiPlatform,
  apiPlatformOptions,
  baseUrl,
  defaultModel,
  keyPlaceholder,
  onSaveKey,
  onApiPlatformChange,
  providerId,
  proxyEnabled,
  setApiKey,
  setBaseUrl,
  setDefaultModel,
  setProxyEnabled,
  setTimeoutSeconds,
  timeoutSeconds,
}: {
  apiKey: string;
  apiKeySaved: boolean;
  apiPlatform?: string;
  apiPlatformOptions?: Array<{ label: string; value: string }>;
  baseUrl?: string | null;
  defaultModel: string | null;
  keyPlaceholder: string;
  onSaveKey: () => void;
  onApiPlatformChange?: (platform: string) => void;
  providerId: (typeof IMAGE_PROVIDER_IDS)[number];
  proxyEnabled: boolean;
  setApiKey: (value: string) => void;
  setBaseUrl: (value: string | null) => void;
  setDefaultModel: (model: string) => void;
  setProxyEnabled: (enabled: boolean) => void;
  setTimeoutSeconds: (value: number) => void;
  timeoutSeconds: number;
}) {
  const provider = getProviderConfig(providerId);
  const models = getProviderModels(providerId, (apiPlatform ?? "openai") as ImageProviderApiPlatform);
  const activeModel = defaultModel ?? models[0]?.id ?? provider.defaults.model;
  const usesHiggsfield = apiPlatform === "higgsfield";

  return (
    <fieldset className="provider-settings">
      <legend>{provider.label}</legend>
      {apiPlatform && onApiPlatformChange && apiPlatformOptions ? (
        <Field label="Platform">
          <select
            value={apiPlatform}
            onChange={(event) => onApiPlatformChange(event.target.value)}
          >
            {apiPlatformOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </Field>
      ) : null}
      {!usesHiggsfield ? (
        <Field label="API Key">
          <div className="template-row">
            <input
              placeholder={
                apiKeySaved
                  ? "Stored in ~/.sozocraft/config.toml"
                  : keyPlaceholder
              }
              type="password"
              value={apiKey}
              onChange={(event) => setApiKey(event.target.value)}
            />
            <button
              className="secondary-button"
              onClick={onSaveKey}
              type="button"
            >
              <KeyRound size={15} />
              Save Key
            </button>
          </div>
        </Field>
      ) : null}
      <div className={`provider-settings-grid${usesHiggsfield ? " higgsfield-grid" : ""}`}>
        {!usesHiggsfield ? (
          <Field label="Base URL">
            <input
              placeholder={
                providerId === "nano-banana"
                  ? "Default Google Generative Language API"
                  : providerId === "gpt-image" && apiPlatform === "openrouter"
                    ? "https://openrouter.ai/api/v1"
                    : providerId === "gpt-image"
                      ? "https://api.openai.com/v1"
                      : undefined
              }
              value={baseUrl ?? ""}
              onChange={(event) => setBaseUrl(event.target.value || null)}
            />
          </Field>
        ) : null}
        <Field label="Timeout">
          <input
            min={10}
            type="number"
            value={timeoutSeconds}
            onChange={(event) => setTimeoutSeconds(Number(event.target.value))}
          />
        </Field>
        <Field label="Default Model" className="field-full">
          <select
            value={activeModel}
            onChange={(event) => setDefaultModel(event.target.value)}
          >
            {models.map((model) => (
              <option key={model.id} value={model.id}>
                {model.productName}
              </option>
            ))}
          </select>
        </Field>
      </div>
      {!usesHiggsfield ? (
        <ToggleSwitch
          checked={proxyEnabled}
          label="Use proxy"
          onChange={setProxyEnabled}
        />
      ) : null}
    </fieldset>
  );
}

function ConfigBadge({
  label,
  missingText = "Missing",
  ok,
  okText = "Configured",
}: {
  label: string;
  missingText?: string;
  ok: boolean;
  okText?: string;
}) {
  return (
    <div className="config-status-row">
      <span className="config-status-label">{label}</span>
      <span className={`config-status-badge ${ok ? "ok" : "missing"}`}>
        {ok ? okText : missingText}
      </span>
    </div>
  );
}

function planNameFromAccount(account?: string | null) {
  const normalized = account?.trim().replace(/\s+/g, " ");
  if (!normalized) {
    return null;
  }
  return normalized
    .replace(/\s+plan$/i, "")
    .split(" ")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join(" ");
}
