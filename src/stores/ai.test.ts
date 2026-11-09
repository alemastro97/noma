import type { AiState } from "../api/types";
import { aiStatusLabel } from "./ai";

const base: AiState = {
  provider: "builtin",
  modelId: "m",
  ollamaModel: null,
  ollamaUrl: "",
  ready: true,
  phase: "idle",
  message: null,
  serverInstalled: true,
  serverRunning: false,
  models: [],
  download: null,
  ollamaAvailable: false,
  ollamaModels: [],
  pending: 0,
  speechModelId: "s",
  speechModels: [],
  speechReady: false,
};

test("status label follows download, phase and readiness", () => {
  expect(aiStatusLabel(null, "idle")).toBe("Idle");
  expect(aiStatusLabel(base, "success")).toBe("Captured");
  expect(
    aiStatusLabel(
      { ...base, download: { modelId: "m", stage: "model", received: 42, total: 100 } },
      "idle",
    ),
  ).toBe("Downloading 42%");
  expect(aiStatusLabel({ ...base, phase: "loading" }, "processing")).toBe("Waking up");
  expect(aiStatusLabel({ ...base, phase: "processing" }, "processing")).toBe("Thinking");
  expect(aiStatusLabel({ ...base, ready: false }, "idle")).toBe("Set up AI");
  expect(aiStatusLabel({ ...base, ready: false, pending: 2 }, "idle")).toBe("2 waiting");
});
