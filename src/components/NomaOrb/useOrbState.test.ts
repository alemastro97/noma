import { act, renderHook } from "@testing-library/react";
import { useOrbState, SUCCESS_FLASH_MS, ERROR_FLASH_MS } from "./useOrbState";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

test("starts idle by default", () => {
  const { result } = renderHook(() => useOrbState());
  expect(result.current.state).toBe("idle");
});

test("flashSuccess returns to idle after 1400ms", () => {
  expect(SUCCESS_FLASH_MS).toBe(1400);
  const { result } = renderHook(() => useOrbState());
  act(() => result.current.flashSuccess());
  expect(result.current.state).toBe("success");
  act(() => vi.advanceTimersByTime(1399));
  expect(result.current.state).toBe("success");
  act(() => vi.advanceTimersByTime(1));
  expect(result.current.state).toBe("idle");
});

test("flashError returns to idle after default 2500ms", () => {
  expect(ERROR_FLASH_MS).toBe(2500);
  const { result } = renderHook(() => useOrbState());
  act(() => result.current.flashError());
  expect(result.current.state).toBe("error");
  act(() => vi.advanceTimersByTime(2500));
  expect(result.current.state).toBe("idle");
});

test("set() cancels a pending flash timer", () => {
  const { result } = renderHook(() => useOrbState());
  act(() => result.current.flashSuccess());
  act(() => result.current.set("processing"));
  act(() => vi.advanceTimersByTime(5000));
  expect(result.current.state).toBe("processing");
});
