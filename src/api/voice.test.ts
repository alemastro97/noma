import { describe, expect, it } from "vitest";
import { voiceCopy } from "../copy";
import { voiceToastKind } from "./voice";

describe("voice extras", () => {
  it("recognizes voice command toasts by id", () => {
    expect(voiceToastKind("voice:done:abc")).toBe("done");
    expect(voiceToastKind("voice:moved:abc")).toBe("moved");
    expect(voiceToastKind("filed:abc")).toBeNull();
    expect(voiceToastKind("brief")).toBeNull();
  });

  it("formats recording durations", () => {
    expect(voiceCopy.duration(400)).toBe("0:01");
    expect(voiceCopy.duration(65_000)).toBe("1:05");
  });
});
