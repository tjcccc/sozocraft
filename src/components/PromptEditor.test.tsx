import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { findSearchMatches, PromptEditor, replaceMatches } from "./PromptEditor";

describe("PromptEditor search and replace", () => {
  it("finds and replaces case-insensitive matches without overlapping ranges", () => {
    const matches = findSearchMatches("Alpha alpha ALPHA", "alpha", false);

    expect(matches).toEqual([
      { start: 0, end: 5 },
      { start: 6, end: 11 },
      { start: 12, end: 17 },
    ]);
    expect(replaceMatches("Alpha alpha ALPHA", matches, "beta")).toBe("beta beta beta");
  });

  it("opens replace from the platform shortcut and replaces all matches", async () => {
    const user = userEvent.setup();
    const { textarea } = renderPromptEditor("alpha beta alpha");

    fireEvent.keyDown(textarea, shortcutEvent("r"));

    const findInput = await screen.findByPlaceholderText("Find");
    const replaceInput = await screen.findByPlaceholderText("Replace");

    expect(findInput.getAttribute("autocapitalize")).toBe("off");
    expect(replaceInput.getAttribute("autocapitalize")).toBe("off");
    expect(findInput.getAttribute("spellcheck")).toBe("false");
    expect(replaceInput.getAttribute("spellcheck")).toBe("false");

    await user.type(findInput, "alpha");
    await user.type(replaceInput, "gamma");
    await user.click(screen.getByRole("button", { name: "All" }));

    await waitFor(() => {
      expect(screen.getByTestId("prompt-value").textContent).toBe("gamma beta gamma");
    });
  });
});

function renderPromptEditor(initialPrompt: string) {
  function Harness() {
    const [prompt, setPrompt] = useState(initialPrompt);
    return (
      <>
        <PromptEditor
          dslEnabled={false}
          getDraggedPromptInclude={() => null}
          highlightRef={{ current: null }}
          onPromptChange={setPrompt}
          prompt={prompt}
        />
        <output data-testid="prompt-value">{prompt}</output>
      </>
    );
  }

  const result = render(<Harness />);
  const textarea = result.container.querySelector<HTMLTextAreaElement>("textarea.prompt-textarea");
  if (!textarea) {
    throw new Error("Prompt textarea was not rendered.");
  }
  return { ...result, textarea };
}

function shortcutEvent(key: "f" | "r") {
  const applePlatform = /mac|iphone|ipad|ipod/i.test(navigator.platform);
  return {
    key,
    ...(applePlatform ? { metaKey: true } : { ctrlKey: true }),
  };
}
