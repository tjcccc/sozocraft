import { KeyRound } from "lucide-react";
import type { AppSettings } from "../types";
import { Field, ToggleSwitch } from "./common";

/**
 * Shared OpenRouter platform settings. Every OpenRouter route (Experimental
 * models and GPT-Image's OpenRouter platform) uses this key, endpoint, proxy,
 * and timeout.
 */
export function OpenRouterSettings({
  apiKey,
  apiKeySaved,
  baseUrl,
  onSaveKey,
  setApiKey,
  setBaseUrl,
  setSettings,
  settings,
}: {
  apiKey: string;
  apiKeySaved: boolean;
  baseUrl: string | null;
  onSaveKey: () => void;
  setApiKey: (value: string) => void;
  setBaseUrl: (value: string | null) => void;
  setSettings: (settings: AppSettings) => void;
  settings: AppSettings;
}) {
  return (
    <fieldset className="provider-settings" id="settings-platform-openrouter">
      <legend>OpenRouter</legend>
      <Field label="API Key">
        <div className="template-row">
          <input
            placeholder={apiKeySaved ? "Stored in ~/.sozocraft/config.toml" : "OpenRouter API key"}
            type="password"
            value={apiKey}
            onChange={(event) => setApiKey(event.target.value)}
          />
          <button className="secondary-button" onClick={onSaveKey} type="button">
            <KeyRound size={15} />
            Save Key
          </button>
        </div>
      </Field>
      <div className="provider-settings-grid">
        <Field label="Base URL">
          <input
            placeholder="https://openrouter.ai/api/v1"
            value={baseUrl ?? ""}
            onChange={(event) => setBaseUrl(event.target.value || null)}
          />
        </Field>
        <Field label="Timeout">
          <input
            min={10}
            type="number"
            value={settings.openrouterTimeoutSeconds}
            onChange={(event) =>
              setSettings({ ...settings, openrouterTimeoutSeconds: Number(event.target.value) })
            }
          />
        </Field>
      </div>
      <ToggleSwitch
        checked={settings.openrouterProxyEnabled}
        label="Use proxy"
        onChange={(checked) => setSettings({ ...settings, openrouterProxyEnabled: checked })}
      />
      <p className="provider-settings-note">
        Used by Experimental models and by GPT-Image when its platform is OpenRouter.
      </p>
    </fieldset>
  );
}
