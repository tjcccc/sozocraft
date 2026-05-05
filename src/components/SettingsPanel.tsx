import { KeyRound } from "lucide-react";
import { useEffect, useRef } from "react";
import type {
  AppSettings,
  ConfigStatus,
  PromptPreviewPlacement,
} from "../types";
import {
  IMAGE_PROVIDER_IDS,
  getProviderConfig,
} from "../models/imageProviders";
import { validateOutputTemplate } from "../utils/outputTemplate";
import { Field, ToggleSwitch } from "./common";

export function SettingsPanel({
  apiKey,
  apiKeySaved,
  openaiApiKey,
  openaiApiKeySaved,
  xaiApiKey,
  xaiApiKeySaved,
  configStatus,
  settings,
  setApiKey,
  setOpenaiApiKey,
  setSettings,
  setXaiApiKey,
  onSaveKey,
  onSaveOpenaiKey,
  onSaveSettings,
  onSaveXaiKey,
}: {
  apiKey: string;
  apiKeySaved: boolean;
  openaiApiKey: string;
  openaiApiKeySaved: boolean;
  xaiApiKey: string;
  xaiApiKeySaved: boolean;
  configStatus: ConfigStatus | null;
  settings: AppSettings;
  setApiKey: (value: string) => void;
  setOpenaiApiKey: (value: string) => void;
  setSettings: (settings: AppSettings) => void;
  setXaiApiKey: (value: string) => void;
  onSaveKey: () => void;
  onSaveOpenaiKey: () => void;
  onSaveSettings: (settings: AppSettings) => void;
  onSaveXaiKey: () => void;
}) {
  const templateIssues = validateOutputTemplate(settings.outputTemplate);
  const didMountRef = useRef(false);

  useEffect(() => {
    if (!didMountRef.current) {
      didMountRef.current = true;
      return;
    }
    const handle = window.setTimeout(() => onSaveSettings(settings), 650);
    return () => window.clearTimeout(handle);
  }, [onSaveSettings, settings]);

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
            <div className="config-status-row">
              <span className="config-status-label">Config file</span>
              <code className="config-status-value">
                {configStatus.configPath}
              </code>
            </div>
            <ConfigBadge label="Gemini API key" ok={configStatus.hasApiKey} />
            <ConfigBadge
              label="OpenAI API key"
              ok={configStatus.hasOpenaiApiKey}
            />
            <ConfigBadge label="xAI API key" ok={configStatus.hasXaiApiKey} />
            <ConfigBadge
              label="Proxy"
              ok={configStatus.hasProxy}
              okText="Configured"
              missingText="Not set"
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
            baseUrl={settings.optionalBaseUrl}
            defaultModel={
              settings.defaultProvider === "nano-banana"
                ? settings.defaultModel
                : null
            }
            keyPlaceholder="Gemini API key"
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
            apiKey={openaiApiKey}
            apiKeySaved={openaiApiKeySaved}
            baseUrl={settings.openaiBaseUrl}
            defaultModel={
              settings.defaultProvider === "gpt-image"
                ? settings.defaultModel
                : null
            }
            keyPlaceholder="OpenAI API key"
            onSaveKey={onSaveOpenaiKey}
            providerId="gpt-image"
            proxyEnabled={settings.openaiProxyEnabled}
            setApiKey={setOpenaiApiKey}
            setBaseUrl={(value) =>
              setSettings({ ...settings, openaiBaseUrl: value })
            }
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
            baseUrl={settings.xaiBaseUrl}
            defaultModel={
              settings.defaultProvider === "grok-imagine"
                ? settings.defaultModel
                : null
            }
            keyPlaceholder="xAI API key"
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
        </div>
      </div>
    </section>
  );
}

function ProviderSettings({
  apiKey,
  apiKeySaved,
  baseUrl,
  defaultModel,
  keyPlaceholder,
  onSaveKey,
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
  baseUrl?: string | null;
  defaultModel: string | null;
  keyPlaceholder: string;
  onSaveKey: () => void;
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
  const activeModel = defaultModel ?? provider.defaults.model;

  return (
    <fieldset className="provider-settings">
      <legend>{provider.providerName}</legend>
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
      <div className="provider-settings-grid">
        <Field label="Base URL">
          <input
            placeholder={
              providerId === "nano-banana"
                ? "Default Google Generative Language API"
                : undefined
            }
            value={baseUrl ?? ""}
            onChange={(event) => setBaseUrl(event.target.value || null)}
          />
        </Field>
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
            {provider.models.map((model) => (
              <option key={model.id} value={model.id}>
                {model.productName}
              </option>
            ))}
          </select>
        </Field>
      </div>
      <ToggleSwitch
        checked={proxyEnabled}
        label="Use proxy"
        onChange={setProxyEnabled}
      />
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
