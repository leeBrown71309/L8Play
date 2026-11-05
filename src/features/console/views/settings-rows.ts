import type { DsIconName } from "@/components/ds/ds-icon";

/** A settings row as the console draws it: a switch, a value picker, a slider or a plain action. */
export type SettingsRow =
  | {
      kind: "toggle";
      id: string;
      label: string;
      detail?: string | undefined;
      icon: DsIconName;
      value: boolean;
      set: (value: boolean) => void;
      disabled?: boolean | undefined;
    }
  | {
      kind: "picker";
      id: string;
      label: string;
      detail?: string | undefined;
      icon: DsIconName;
      options: readonly { value: string; label: string }[];
      value: string;
      set: (value: string) => void;
    }
  | {
      kind: "slider";
      id: string;
      label: string;
      icon: DsIconName;
      value: number;
      set: (value: number) => void;
      disabled?: boolean | undefined;
    }
  | {
      kind: "action";
      id: string;
      label: string;
      detail?: string | undefined;
      icon: DsIconName;
      run: () => void;
      disabled?: boolean | undefined;
    };

/** The option after (or before) the current one, wrapping around the ends. */
export function cycleOption<T extends { value: string }>(
  options: readonly T[],
  current: string,
  delta: 1 | -1,
): T | undefined {
  if (options.length === 0) return undefined;
  const index = options.findIndex((option) => option.value === current);
  return options[(Math.max(0, index) + delta + options.length) % options.length];
}

/** A volume step of one tenth, kept between 0 and 1 without floating point crumbs. */
export function stepVolume(current: number, delta: 1 | -1): number {
  return Math.min(1, Math.max(0, Math.round((current + delta * 0.1) * 10) / 10));
}
