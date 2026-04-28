import { useCallback, useEffect, useState } from "react";
import {
  getConfigStatus,
  hasGeminiApiKey,
  loadAppState,
  saveAppSettings,
  setGeminiApiKey,
} from "../api";
import type { AppSettings, ConfigStatus, GenerationBatch } from "../types";
import type { AppStatus } from "../components/common";

export function useAppState() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [prompt, setPrompt] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [apiKeySaved, setApiKeySaved] = useState(false);
  const [batches, setBatches] = useState<GenerationBatch[]>([]);
  const [status, setStatus] = useState<AppStatus>("ready");
  const [message, setMessage] = useState("Ready");
  const [configStatus, setConfigStatus] = useState<ConfigStatus | null>(null);

  useEffect(() => {
    void loadAppState()
      .then((state) => {
        setSettings(state.settings);
        setPrompt(state.currentPrompt);
        setBatches(state.batches);
        setMessage("Ready");
      })
      .catch((error) => {
        setStatus("error");
        setMessage(String(error));
      });

    void hasGeminiApiKey()
      .then(setApiKeySaved)
      .catch(() => setApiKeySaved(false));

    void getConfigStatus()
      .then(setConfigStatus)
      .catch(() => undefined);
  }, []);

  const updateSettings = useCallback(async (next: AppSettings) => {
    setSettings(next);
    const state = await saveAppSettings(next);
    setBatches(state.batches);
    setMessage("Settings saved");
    setStatus("ready");
  }, []);

  const saveKey = useCallback(async () => {
    try {
      await setGeminiApiKey(apiKey);
      setApiKey("");
      setApiKeySaved(apiKey.trim().length > 0);
      setStatus("ready");
      setMessage(apiKey.trim().length > 0 ? "API key saved" : "API key cleared");
      void getConfigStatus().then(setConfigStatus).catch(() => undefined);
    } catch (error) {
      setStatus("error");
      setMessage(String(error));
    }
  }, [apiKey]);

  return {
    apiKey,
    apiKeySaved,
    batches,
    configStatus,
    message,
    prompt,
    settings,
    status,
    saveKey,
    setApiKey,
    setBatches,
    setMessage,
    setPrompt,
    setSettings,
    setStatus,
    updateSettings,
  };
}
