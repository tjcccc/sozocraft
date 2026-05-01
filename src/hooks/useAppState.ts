import { useCallback, useEffect, useState } from "react";
import {
  getConfigStatus,
  hasGeminiApiKey,
  hasOpenaiApiKey,
  hasXaiApiKey,
  loadAppState,
  saveAppSettings,
  setGeminiApiKey,
  setOpenaiApiKey as persistOpenaiApiKey,
  setXaiApiKey as persistXaiApiKey,
} from "../api";
import type { AppSettings, ConfigStatus, GenerationBatch } from "../types";
import type { AppStatus } from "../components/common";

export function useAppState() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [prompt, setPrompt] = useState("");
  const [currentPromptId, setCurrentPromptId] = useState<string | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [openaiApiKey, setOpenaiApiKey] = useState("");
  const [xaiApiKey, setXaiApiKey] = useState("");
  const [apiKeySaved, setApiKeySaved] = useState(false);
  const [openaiApiKeySaved, setOpenaiApiKeySaved] = useState(false);
  const [xaiApiKeySaved, setXaiApiKeySaved] = useState(false);
  const [batches, setBatches] = useState<GenerationBatch[]>([]);
  const [status, setStatus] = useState<AppStatus>("ready");
  const [message, setMessage] = useState("Ready");
  const [configStatus, setConfigStatus] = useState<ConfigStatus | null>(null);

  useEffect(() => {
    void loadAppState()
      .then((state) => {
        setSettings(state.settings);
        setPrompt(state.currentPrompt);
        setCurrentPromptId(state.currentPromptId ?? null);
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
    void hasOpenaiApiKey()
      .then(setOpenaiApiKeySaved)
      .catch(() => setOpenaiApiKeySaved(false));
    void hasXaiApiKey()
      .then(setXaiApiKeySaved)
      .catch(() => setXaiApiKeySaved(false));

    void getConfigStatus()
      .then(setConfigStatus)
      .catch(() => undefined);
  }, []);

  const updateSettings = useCallback(async (next: AppSettings) => {
    try {
      if (apiKey.trim()) {
        await setGeminiApiKey(apiKey);
        setApiKey("");
        setApiKeySaved(true);
      }
      if (openaiApiKey.trim()) {
        await persistOpenaiApiKey(openaiApiKey);
        setOpenaiApiKey("");
        setOpenaiApiKeySaved(true);
      }
      if (xaiApiKey.trim()) {
        await persistXaiApiKey(xaiApiKey);
        setXaiApiKey("");
        setXaiApiKeySaved(true);
      }

      setSettings(next);
      const state = await saveAppSettings(next);
      setBatches(state.batches);
      setMessage("Settings saved");
      setStatus("ready");
      void getConfigStatus().then(setConfigStatus).catch(() => undefined);
    } catch (error) {
      setStatus("error");
      setMessage(String(error));
    }
  }, [apiKey, openaiApiKey, xaiApiKey]);

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

  const saveOpenaiKey = useCallback(async () => {
    try {
      await persistOpenaiApiKey(openaiApiKey);
      setOpenaiApiKey("");
      setOpenaiApiKeySaved(openaiApiKey.trim().length > 0);
      setStatus("ready");
      setMessage(openaiApiKey.trim().length > 0 ? "OpenAI API key saved" : "OpenAI API key cleared");
      void getConfigStatus().then(setConfigStatus).catch(() => undefined);
    } catch (error) {
      setStatus("error");
      setMessage(String(error));
    }
  }, [openaiApiKey]);

  const saveXaiKey = useCallback(async () => {
    try {
      await persistXaiApiKey(xaiApiKey);
      setXaiApiKey("");
      setXaiApiKeySaved(xaiApiKey.trim().length > 0);
      setStatus("ready");
      setMessage(xaiApiKey.trim().length > 0 ? "xAI API key saved" : "xAI API key cleared");
      void getConfigStatus().then(setConfigStatus).catch(() => undefined);
    } catch (error) {
      setStatus("error");
      setMessage(String(error));
    }
  }, [xaiApiKey]);

  return {
    apiKey,
    apiKeySaved,
    openaiApiKey,
    openaiApiKeySaved,
    xaiApiKey,
    xaiApiKeySaved,
    batches,
    configStatus,
    message,
    prompt,
    currentPromptId,
    settings,
    status,
    saveKey,
    saveOpenaiKey,
    saveXaiKey,
    setApiKey,
    setOpenaiApiKey,
    setXaiApiKey,
    setBatches,
    setMessage,
    setPrompt,
    setCurrentPromptId,
    setSettings,
    setStatus,
    updateSettings,
  };
}
