import { useCallback, useEffect, useState } from "react";
import {
  getConfigStatus,
  hasArkApiKey,
  hasGeminiApiKey,
  hasOpenaiApiKey,
  hasOpenrouterApiKey,
  hasXaiApiKey,
  loadAppState,
  saveAppSettings,
  setGeminiApiKey,
  setArkApiKey as persistArkApiKey,
  setOpenaiApiKey as persistOpenaiApiKey,
  setOpenrouterApiKey as persistOpenrouterApiKey,
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
  const [openrouterApiKey, setOpenrouterApiKey] = useState("");
  const [xaiApiKey, setXaiApiKey] = useState("");
  const [arkApiKey, setArkApiKey] = useState("");
  const [apiKeySaved, setApiKeySaved] = useState(false);
  const [openaiApiKeySaved, setOpenaiApiKeySaved] = useState(false);
  const [openrouterApiKeySaved, setOpenrouterApiKeySaved] = useState(false);
  const [xaiApiKeySaved, setXaiApiKeySaved] = useState(false);
  const [arkApiKeySaved, setArkApiKeySaved] = useState(false);
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
    void hasOpenrouterApiKey()
      .then(setOpenrouterApiKeySaved)
      .catch(() => setOpenrouterApiKeySaved(false));
    void hasXaiApiKey()
      .then(setXaiApiKeySaved)
      .catch(() => setXaiApiKeySaved(false));
    void hasArkApiKey()
      .then(setArkApiKeySaved)
      .catch(() => setArkApiKeySaved(false));

    void getConfigStatus()
      .then(setConfigStatus)
      .catch(() => undefined);
  }, []);

  const updateSettings = useCallback(async (next: AppSettings) => {
    try {
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

  const saveOpenrouterKey = useCallback(async () => {
    try {
      await persistOpenrouterApiKey(openrouterApiKey);
      setOpenrouterApiKey("");
      setOpenrouterApiKeySaved(openrouterApiKey.trim().length > 0);
      setStatus("ready");
      setMessage(openrouterApiKey.trim().length > 0 ? "OpenRouter API key saved" : "OpenRouter API key cleared");
      void getConfigStatus().then(setConfigStatus).catch(() => undefined);
    } catch (error) {
      setStatus("error");
      setMessage(String(error));
    }
  }, [openrouterApiKey]);

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

  const saveArkKey = useCallback(async () => {
    try {
      await persistArkApiKey(arkApiKey);
      setArkApiKey("");
      setArkApiKeySaved(arkApiKey.trim().length > 0);
      setStatus("ready");
      setMessage(
        arkApiKey.trim().length > 0
          ? "Volcengine Ark API key saved"
          : "Volcengine Ark API key cleared",
      );
      void getConfigStatus().then(setConfigStatus).catch(() => undefined);
    } catch (error) {
      setStatus("error");
      setMessage(String(error));
    }
  }, [arkApiKey]);

  return {
    apiKey,
    apiKeySaved,
    arkApiKey,
    arkApiKeySaved,
    openaiApiKey,
    openaiApiKeySaved,
    openrouterApiKey,
    openrouterApiKeySaved,
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
    saveArkKey,
    saveOpenaiKey,
    saveOpenrouterKey,
    saveXaiKey,
    setApiKey,
    setArkApiKey,
    setOpenaiApiKey,
    setOpenrouterApiKey,
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
