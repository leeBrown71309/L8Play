import { useLingui } from "@lingui/react/macro";

export type DayPeriod = "morning" | "afternoon" | "evening";

/** Morning 5–11 h, afternoon 12–17 h, evening otherwise. */
export function dayPeriod(date: Date): DayPeriod {
  const hour = date.getHours();
  if (hour >= 5 && hour < 12) return "morning";
  if (hour >= 12 && hour < 18) return "afternoon";
  return "evening";
}

/** Dashboard greeting that follows the time of day (`l8-greeting`). */
export function Greeting({ now = new Date() }: { now?: Date }) {
  const { t } = useLingui();
  const text = { morning: t`Good morning`, afternoon: t`Good afternoon`, evening: t`Good evening` }[dayPeriod(now)];
  return (
    <h1 className="l8-greeting">
      <b>{text}</b>
    </h1>
  );
}
