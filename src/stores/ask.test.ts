import { HISTORY_TURNS, historyOf, type Turn } from "./ask";

const answered = (id: number): Turn => ({
  id,
  question: `q${id}`,
  pending: false,
  result: { answer: `a${id}`, cards: [], cited: [] },
});

test("history keeps the last answered turns only", () => {
  const turns: Turn[] = [
    answered(1),
    answered(2),
    { id: 3, question: "failed", pending: false, error: "boom" },
    answered(4),
    answered(5),
    { id: 6, question: "waiting", pending: true },
  ];
  const history = historyOf(turns);
  expect(history).toHaveLength(HISTORY_TURNS);
  expect(history.map((t) => t.question)).toEqual(["q2", "q4", "q5"]);
});
