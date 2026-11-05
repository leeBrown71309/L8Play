import { describe, expect, it } from "vitest";

import { AppError, toAppError } from "@/lib/tauri/errors";

describe("toAppError", () => {
  it("keeps the code of a backend CommandError", () => {
    const error = toAppError({ code: "database", message: "disk I/O error" });
    expect(error).toBeInstanceOf(AppError);
    expect(error.code).toBe("database");
    expect(error.message).toBe("disk I/O error");
  });

  it("wraps unknown failures as internal errors", () => {
    expect(toAppError(new Error("boom")).code).toBe("internal");
    expect(toAppError("plain string").message).toBe("plain string");
  });

  it("returns an existing AppError unchanged", () => {
    const original = new AppError("io", "denied");
    expect(toAppError(original)).toBe(original);
  });
});
