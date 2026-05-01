import { KeyRound, Save } from "lucide-react";
import type { AppSettings, ConfigStatus } from "../types";
import { Field } from "./common";

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
  onSaveSettings: () => void;
  onSaveXaiKey: () => void;
}) {
  return (
    <section className="settings-panel">
      <div className="settings-section">
        <h2>Provider Keys</h2>
        <Field label="Gemini API Key">
          <div className="template-row">
            <input
              placeholder={apiKeySaved ? "Stored in ~/.sozocraft/config.toml" : "Paste API key"}
              type="password"
              value={apiKey}
              onChange={(event) => setApiKey(event.target.value)}
            />
            <button className="secondary-button" onClick={onSaveKey}>
              <KeyRound size={15} />
              Save Key
            </button>
          </div>
        </Field>
        <Field label="OpenAI API Key">
          <div className="template-row">
            <input
              placeholder={openaiApiKeySaved ? "Stored in ~/.sozocraft/config.toml" : "Paste API key"}
              type="password"
              value={openaiApiKey}
              onChange={(event) => setOpenaiApiKey(event.target.value)}
            />
            <button className="secondary-button" onClick={onSaveOpenaiKey}>
              <KeyRound size={15} />
              Save Key
            </button>
          </div>
        </Field>
        <Field label="xAI API Key">
          <div className="template-row">
            <input
              placeholder={xaiApiKeySaved ? "Stored in ~/.sozocraft/config.toml" : "Paste API key"}
              type="password"
              value={xaiApiKey}
              onChange={(event) => setXaiApiKey(event.target.value)}
            />
            <button className="secondary-button" onClick={onSaveXaiKey}>
              <KeyRound size={15} />
              Save Key
            </button>
          </div>
        </Field>
      </div>
      <div className="settings-section">
        <h2>Output</h2>
        <Field label="Output Directory">
          <input
            value={settings.outputDirectory}
            onChange={(event) => setSettings({ ...settings, outputDirectory: event.target.value })}
          />
        </Field>
        <Field label="Prompt Directory">
          <input
            value={settings.promptDirectory}
            onChange={(event) => setSettings({ ...settings, promptDirectory: event.target.value })}
          />
        </Field>
      </div>
      <div className="settings-section">
        <h2>Provider Endpoints</h2>
        <Field label="Gemini Base URL">
          <input
            placeholder="Default Google Generative Language API"
            value={settings.optionalBaseUrl ?? ""}
            onChange={(event) =>
              setSettings({ ...settings, optionalBaseUrl: event.target.value || null })
            }
          />
        </Field>
        <Field label="OpenAI Base URL">
          <input
            placeholder="https://api.openai.com/v1"
            value={settings.openaiBaseUrl ?? ""}
            onChange={(event) =>
              setSettings({ ...settings, openaiBaseUrl: event.target.value || null })
            }
          />
        </Field>
        <Field label="xAI Base URL">
          <input
            placeholder="https://api.x.ai/v1"
            value={settings.xaiBaseUrl ?? ""}
            onChange={(event) =>
              setSettings({ ...settings, xaiBaseUrl: event.target.value || null })
            }
          />
        </Field>
        <Field label="Proxy URL">
          <input
            placeholder="http://127.0.0.1:7890"
            value={settings.proxyUrl ?? ""}
            onChange={(event) => setSettings({ ...settings, proxyUrl: event.target.value || null })}
          />
        </Field>
      </div>
      <div className="settings-section">
        <h2>Runtime</h2>
        <Field label="Timeout">
          <input
            min={10}
            type="number"
            value={settings.timeoutSeconds}
            onChange={(event) =>
              setSettings({ ...settings, timeoutSeconds: Number(event.target.value) })
            }
          />
        </Field>
        <button className="primary-button" onClick={onSaveSettings}>
          <Save size={15} />
          Save Settings
        </button>
      </div>
      {configStatus ? (
        <div className="config-status">
          <div className="config-status-row">
            <span className="config-status-label">Config file</span>
            <code className="config-status-value">{configStatus.configPath}</code>
          </div>
          <div className="config-status-row">
            <span className="config-status-label">Gemini API key</span>
            <span className={`config-status-badge ${configStatus.hasApiKey ? "ok" : "missing"}`}>
              {configStatus.hasApiKey ? "Configured" : "Missing"}
            </span>
          </div>
          <div className="config-status-row">
            <span className="config-status-label">OpenAI API key</span>
            <span className={`config-status-badge ${configStatus.hasOpenaiApiKey ? "ok" : "missing"}`}>
              {configStatus.hasOpenaiApiKey ? "Configured" : "Missing"}
            </span>
          </div>
          <div className="config-status-row">
            <span className="config-status-label">xAI API key</span>
            <span className={`config-status-badge ${configStatus.hasXaiApiKey ? "ok" : "missing"}`}>
              {configStatus.hasXaiApiKey ? "Configured" : "Missing"}
            </span>
          </div>
          <div className="config-status-row">
            <span className="config-status-label">Proxy</span>
            <span className={`config-status-badge ${configStatus.hasProxy ? "ok" : "missing"}`}>
              {configStatus.hasProxy ? "Configured" : "Not set"}
            </span>
          </div>
        </div>
      ) : null}
    </section>
  );
}
