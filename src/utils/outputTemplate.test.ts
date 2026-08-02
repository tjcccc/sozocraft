import { describe, expect, it } from "vitest";
import { validateOutputTemplate } from "./outputTemplate";

describe("validateOutputTemplate", () => {
  it("accepts configurable ID widths", () => {
    expect(validateOutputTemplate("{id:2}_{id:4}.{extension}")).toEqual([]);
  });

  it("accepts the original Higgsfield filename only for Higgsfield templates", () => {
    const template = "{yyMMdd} {id:3} {higgsfield_filename}";

    expect(validateOutputTemplate(template, { higgsfield: true })).toEqual([]);
    expect(validateOutputTemplate(template)).toContain(
      "{higgsfield_filename} is not a recognised token and will render literally.",
    );
  });

  it("rejects unsupported ID widths", () => {
    expect(validateOutputTemplate("{id:0}")).toContain(
      "{id:0} — ID width must be between 1 and 12.",
    );
  });
});
