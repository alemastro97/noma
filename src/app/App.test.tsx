import { render, screen } from "@testing-library/react";
import { App } from "./App";

test("renders the NOMA shell root", () => {
  render(<App />);
  expect(screen.getByRole("application", { name: "NOMA" })).toBeInTheDocument();
});
