import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useQuickPrompts } from "../hooks/useQuickPrompts";
import { QuickPromptPanel } from "./QuickPromptPanel";

vi.mock("@tauri-apps/api/core", () => ({ invoke: (command: string) => Promise.resolve(command === "load_quick_prompts" ? {
  enabled: true, pending: false, activeId: "one", title: "", tags: "",
  tabs: [{ id: "one", number: 1, source: "# plain text" }],
} : undefined) }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ onCloseRequested: () => Promise.resolve(() => undefined) }) }));
afterEach(cleanup);

it("disables DSL, hides metadata, confirms nonempty closes, and offers explicit library save", async () => {
  const save = vi.fn().mockResolvedValue(undefined);
  function Harness() {
    const quick = useQuickPrompts(vi.fn());
    return quick.state ? <QuickPromptPanel quick={quick} onSave={save} /> : null;
  }
  render(<Harness />);
  await screen.findByRole("tab", { name: "Prompt 1" });
  expect((screen.getByRole("checkbox", { name: "DSL" }) as HTMLInputElement).disabled).toBe(true);
  expect(screen.queryByLabelText("Prompt name")).toBeNull();
  expect(screen.queryByLabelText("Tags")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Close Prompt 1" }));
  expect(screen.getByRole("alertdialog")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Keep" }));
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe("# plain text");
  fireEvent.click(screen.getByRole("button", { name: "Save to Library…" }));
  expect(screen.getByLabelText("Prompt name")).toBeTruthy();
  expect(save).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText("Prompt name"), { target: { value: "My prompt" } });
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Save to Library" })));
  await waitFor(() => expect(save).toHaveBeenCalledWith("My prompt", "# plain text", []));
});

it("switches tabs with arrows and disables navigation at the ends", async () => {
  function Harness() {
    const quick = useQuickPrompts(vi.fn());
    return quick.state ? <QuickPromptPanel quick={quick} onSave={vi.fn()} /> : null;
  }
  render(<Harness />);
  await screen.findByRole("tab", { name: "Prompt 1" });
  const previous = screen.getByRole("button", { name: "Previous Quick prompt" }) as HTMLButtonElement;
  const next = screen.getByRole("button", { name: "Next Quick prompt" }) as HTMLButtonElement;
  expect(previous.disabled).toBe(true);
  expect(next.disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Add Quick prompt" }));
  expect(screen.getByRole("tab", { name: "Prompt 2" }).getAttribute("aria-selected")).toBe("true");
  expect(previous.disabled).toBe(false);
  expect(next.disabled).toBe(true);
  fireEvent.click(previous);
  expect(screen.getByRole("tab", { name: "Prompt 1" }).getAttribute("aria-selected")).toBe("true");
  expect(previous.disabled).toBe(true);
  fireEvent.click(next);
  expect(screen.getByRole("tab", { name: "Prompt 2" }).getAttribute("aria-selected")).toBe("true");
  await waitFor(() => expect(screen.getByRole("status").textContent).toBe("Saved"));
});

it("restores an imported replacement using Cmd+Z in the editor", async () => {
  function Harness() {
    const quick = useQuickPrompts(vi.fn());
    return quick.state ? <>
      <button onClick={() => {
        for (let i = 0; i < 7; i++) quick.add();
        quick.setSource("original eighth prompt");
        quick.importSource("image metadata prompt");
      }}>Import at capacity</button>
      <QuickPromptPanel quick={quick} onSave={vi.fn()} />
    </> : null;
  }
  render(<Harness />);
  await screen.findByRole("tab", { name: "Prompt 1" });
  fireEvent.click(screen.getByRole("button", { name: "Import at capacity" }));
  const editor = screen.getByRole("textbox") as HTMLTextAreaElement;
  expect(editor.value).toBe("image metadata prompt");
  fireEvent.keyDown(editor, { key: "z", metaKey: true });
  expect(editor.value).toBe("original eighth prompt");
  fireEvent.keyDown(editor, { key: "z", metaKey: true, shiftKey: true });
  expect(editor.value).toBe("image metadata prompt");
  await waitFor(() => expect(screen.getByRole("status").textContent).toBe("Saved"));
});
