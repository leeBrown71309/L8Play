import { useEffect, useState } from "react";

const MINUTE_MS = 60_000;

/** The time of day as shown in the top bar ("21:12"), refreshed on the minute. */
export function useConsoleClock(locale: string): string {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    let timer = 0;
    const schedule = () => {
      timer = window.setTimeout(
        () => {
          setNow(new Date());
          schedule();
        },
        MINUTE_MS - (Date.now() % MINUTE_MS) + 50,
      );
    };
    schedule();
    return () => window.clearTimeout(timer);
  }, []);
  return new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" }).format(now);
}
