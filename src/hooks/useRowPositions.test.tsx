import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { RowPositions } from "../api";
import { rowKeyOf } from "./expansionState";
import { useRowPositions, type RowPositionsAnswer } from "./useRowPositions";

vi.mock("../api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api")>();
  return { ...original, rowPositions: vi.fn() };
});

const api = await import("../api");

const a = rowKeyOf("stream-a", 1);
const b = rowKeyOf("stream\u0000b", 7);

function positions(values: Array<number | null>, versions: Partial<RowPositions> = {}) {
  return {
    positions: values,
    timelineVersion: 1,
    resultVersion: 0,
    discardGeneration: 0,
    totalCount: 10,
    ...versions,
  };
}

/** Holds the calls to `rowPositions` until the test answers them. */
function deferredCalls() {
  const replies: Array<{
    resolve: (answer: RowPositions) => void;
    reject: (error: unknown) => void;
  }> = [];
  vi.mocked(api.rowPositions).mockImplementation(
    () =>
      new Promise<RowPositions>((resolve, reject) => {
        replies.push({ resolve, reject });
      }),
  );
  return replies;
}

function setup(keys: { current: string[] }) {
  const answers: RowPositionsAnswer[] = [];
  const onError = vi.fn();
  const { result } = renderHook(() =>
    useRowPositions({
      getKeys: () => keys.current,
      onAnswer: (answer) => answers.push(answer),
      onError,
    }),
  );
  return { request: () => result.current(), answers, onError };
}

describe("useRowPositions", () => {
  beforeEach(() => {
    vi.mocked(api.rowPositions).mockReset();
  });

  it("asks for every key in one call and answers by key", async () => {
    vi.mocked(api.rowPositions).mockResolvedValue(positions([4, null]));
    const { request, answers } = setup({ current: [a, b, a] });
    act(() => request());
    await waitFor(() => expect(answers).toHaveLength(1));
    expect(api.rowPositions).toHaveBeenCalledTimes(1);
    expect(api.rowPositions).toHaveBeenCalledWith([
      { logStreamName: "stream-a", sequence: 1 },
      { logStreamName: "stream\u0000b", sequence: 7 },
    ]);
    expect([...answers[0]!.positions]).toEqual([
      [a, 4],
      [b, null],
    ]);
    expect(answers[0]!.totalCount).toBe(10);
  });

  it("calls nothing without keys", () => {
    const { request } = setup({ current: ["not a key"] });
    act(() => request());
    expect(api.rowPositions).not.toHaveBeenCalled();
  });

  it("keeps one call in flight and sends a request made meanwhile afterwards with the keys of then", async () => {
    const replies = deferredCalls();
    const keys = { current: [a] };
    const { request, answers } = setup(keys);
    act(() => request());
    keys.current = [a, b];
    act(() => request());
    act(() => request());
    expect(api.rowPositions).toHaveBeenCalledTimes(1);
    await act(async () => replies[0]!.resolve(positions([1])));
    await waitFor(() => expect(api.rowPositions).toHaveBeenCalledTimes(2));
    expect(vi.mocked(api.rowPositions).mock.calls[1]![0]).toHaveLength(2);
    await act(async () => replies[1]!.resolve(positions([2, 3], { timelineVersion: 2 })));
    expect(answers.map((answer) => answer.timelineVersion)).toEqual([1, 2]);
  });

  it("drops an answer older than one already seen", async () => {
    vi.mocked(api.rowPositions)
      .mockResolvedValueOnce(positions([5], { timelineVersion: 3, discardGeneration: 1 }))
      .mockResolvedValueOnce(positions([4], { timelineVersion: 2, discardGeneration: 1 }))
      .mockResolvedValueOnce(positions([6], { timelineVersion: 3, discardGeneration: 0 }));
    const { request, answers } = setup({ current: [a] });
    for (let call = 1; call <= 3; call += 1) {
      act(() => request());
      await waitFor(() => expect(api.rowPositions).toHaveBeenCalledTimes(call));
      await act(async () => undefined);
    }
    expect(answers.map((answer) => answer.positions.get(a))).toEqual([5]);
  });

  it("reports an error and still sends the request made meanwhile", async () => {
    const replies = deferredCalls();
    const { request, onError, answers } = setup({ current: [a] });
    act(() => request());
    act(() => request());
    await act(async () => replies[0]!.reject({ key: "session.busy" }));
    expect(onError).toHaveBeenCalledWith({ key: "session.busy" });
    await waitFor(() => expect(api.rowPositions).toHaveBeenCalledTimes(2));
    await act(async () => replies[1]!.resolve(positions([0])));
    expect(answers).toHaveLength(1);
  });
});
