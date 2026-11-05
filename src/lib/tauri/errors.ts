import type { CommandError } from "@/lib/bindings/CommandError";
import type { CommandErrorCode } from "@/lib/bindings/CommandErrorCode";

/** A failed backend command, with the stable code the UI uses to pick a translated message. */
export class AppError extends Error {
  readonly code: CommandErrorCode;

  constructor(code: CommandErrorCode, message: string) {
    super(message);
    this.name = "AppError";
    this.code = code;
  }
}

function isCommandError(value: unknown): value is CommandError {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}

/** Normalizes anything thrown by `invoke` into an {@link AppError}. */
export function toAppError(error: unknown): AppError {
  if (error instanceof AppError) return error;
  if (isCommandError(error)) return new AppError(error.code, error.message);
  return new AppError("internal", error instanceof Error ? error.message : String(error));
}
