import { KeyRound, Save } from "lucide-react";
import type { AppSettings, ConfigStatus } from "../types";
import { Field } from "./common";

export function SettingsPanel({
  apiKey,
  apiKeySaved,
  configStatus,
  settings,
  setApiKey,
  setSettings,
  onSaveKey,
  onSaveSettings,
}: {
  apiKey: string;
  apiKeySaved: boolean;
  configStatus: ConfigStatus | null;
  settings: AppSettings;
  setApiKey: (value: string) => void;
  setSettings: (settings: AppSettings) => void;
  onSaveKey: () => void;
  onSaveSettings: () => void;
}) {
  return (
    <section className="settings-panel">
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
      <Field label="Output Directory">
        <input
          value={settings.outputDirectory}
          onChange={(event) => setSettings({ ...settings, outputDirectory: event.target.value })}
        />
      </Field>
      <Field label="Base URL">
        <input
          placeholder="Default Google Generative Language API"
          value={settings.optionalBaseUrl ?? ""}
          onChange={(event) =>
            setSettings({ ...settings, optionalBaseUrl: event.target.value || null })
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
