import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useQuickPrompts, type QuickState } from "./useQuickPrompts";

const invoke = vi.hoisted(() => vi.fn());
const native = vi.hoisted(() => ({ destroy: vi.fn().mockResolvedValue(undefined), close: undefined as undefined | ((event: { preventDefault: () => void }) => Promise<void>) }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ destroy: native.destroy, onCloseRequested: (handler: typeof native.close) => { native.close = handler; return Promise.resolve(() => undefined); } }) }));
const initial = (): QuickState => ({ enabled: true, pending: false, activeId: "one", title: "", tags: "", tabs: [{ id: "one", number: 1, source: "# shared prompt" }] });
beforeEach(() => { invoke.mockImplementation((command: string) => Promise.resolve(command === "load_quick_prompts" ? initial() : undefined)); });
afterEach(() => { cleanup(); vi.clearAllMocks(); });

async function setup() {
  const error = vi.fn();
  const hook = renderHook(() => useQuickPrompts(error));
  await waitFor(() => expect(hook.result.current.state).not.toBeNull());
  return { ...hook, error };
}

describe("Quick workspace", () => {
  it("restores tabs and preserves unsaved text through mode switches", async () => {
    const { result } = await setup();
    expect(result.current.active?.source).toBe("# shared prompt");
    act(() => result.current.toggle(false));
    expect(result.current.state?.pending).toBe(true);
    act(() => result.current.setSource("new # literal text"));
    act(() => result.current.toggle(true));
    expect(result.current.active?.source).toBe("new # literal text");
    await act(() => result.current.flush());
    const snapshot = invoke.mock.calls.filter(c => c[0] === "save_quick_prompts").slice(-1)[0]?.[1].state;
    expect(snapshot.enabled).toBe(true);
    expect(snapshot.tabs[0].source).toBe("new # literal text");
    expect(invoke.mock.calls.every(c => ["load_quick_prompts", "save_quick_prompts"].includes(c[0]))).toBe(true);
  });

  it("limits tabs to eight, preserves selection, and leaves one empty tab", async () => {
    const { result } = await setup();
    act(() => { for (let i = 0; i < 10; i++) result.current.add(); });
    expect(result.current.state?.tabs).toHaveLength(8);
    expect(new Set(result.current.state?.tabs.map(t => t.number)).size).toBe(8);
    act(() => result.current.select("one"));
    expect(result.current.active?.source).toBe("# shared prompt");
    const ids = result.current.state!.tabs.map(t => t.id);
    act(() => ids.forEach(id => result.current.close(id)));
    expect(result.current.state?.tabs).toHaveLength(1);
    expect(result.current.active?.source).toBe("");
    await act(() => result.current.flush());
  });

  it("clears only the transferred tab after successful library save", async () => {
    const { result } = await setup();
    act(() => result.current.add());
    act(() => result.current.setSource("second"));
    act(() => result.current.select("one"));
    act(() => result.current.toggle(false));
    act(() => { result.current.setTitle("Saved prompt"); result.current.setTags("#portrait #test"); });
    const save = vi.fn().mockResolvedValue(undefined);
    await act(() => result.current.saveToLibrary(save));
    expect(save).toHaveBeenCalledWith("Saved prompt", "# shared prompt", ["portrait", "test"]);
    expect(result.current.editing).toBe(false);
    act(() => result.current.toggle(true));
    expect(result.current.active?.source).toBe("");
    expect(result.current.state?.tabs[1].source).toBe("second");
    await act(() => result.current.flush());
  });

  it("retains the draft when library save fails", async () => {
    const { result, error } = await setup();
    act(() => result.current.toggle(false));
    await act(() => result.current.saveToLibrary(vi.fn().mockRejectedValue(new Error("disk full"))));
    expect(result.current.active?.source).toBe("# shared prompt");
    expect(result.current.state?.pending).toBe(true);
    expect(error).toHaveBeenCalled();
  });

  it("shows persistence errors and supports retry without dropping text", async () => {
    const { result } = await setup();
    invoke.mockImplementation((command: string) => command === "save_quick_prompts" ? Promise.reject(new Error("disk full")) : Promise.resolve(initial()));
    act(() => result.current.setSource("keep this"));
    await act(async () => { await expect(result.current.flush()).rejects.toThrow("disk full"); });
    expect(result.current.saveState).toBe("error");
    expect(result.current.active?.source).toBe("keep this");
    invoke.mockResolvedValue(undefined);
    await act(() => result.current.flush());
    expect(result.current.saveState).toBe("saved");
  });
  it("flushes the latest text before closing and keeps the window open on failure", async () => {
    const { result } = await setup();
    act(() => result.current.setSource("last keystroke"));
    const preventDefault = vi.fn();
    await act(async () => { await native.close!({ preventDefault }); });
    expect(preventDefault).toHaveBeenCalled();
    expect(native.destroy).toHaveBeenCalledTimes(1);
    expect(invoke.mock.calls.filter(c => c[0] === "save_quick_prompts").slice(-1)[0][1].state.tabs[0].source).toBe("last keystroke");
    native.destroy.mockClear();
    invoke.mockRejectedValue(new Error("disk full"));
    await act(async () => { await native.close!({ preventDefault }); });
    expect(native.destroy).not.toHaveBeenCalled();
    expect(result.current.saveState).toBe("error");
  });

});
