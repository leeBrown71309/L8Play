import { afterEach, describe, expect, it } from "vitest";

import { toImageSrc } from "@/lib/tauri/image-src";

const CACHED = "C:\\Users\\Ryu\\AppData\\Local\\com.l8play.app\\artwork\\5\\cover-36aebf0c3dee7a41.jpg";

describe("toImageSrc", () => {
  afterEach(() => {
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  });

  it("leaves remote and inline images untouched", () => {
    expect(toImageSrc("https://cdn.cloudflare.steamstatic.com/steam/apps/1/header.jpg")).toBe(
      "https://cdn.cloudflare.steamstatic.com/steam/apps/1/header.jpg",
    );
    expect(toImageSrc("data:image/svg+xml;utf8,%3Csvg%3E")).toBe("data:image/svg+xml;utf8,%3Csvg%3E");
    expect(toImageSrc("blob:http://localhost/abc")).toBe("blob:http://localhost/abc");
  });

  it("passes local paths through outside the desktop app (browser preview, tests)", () => {
    expect(toImageSrc(CACHED)).toBe(CACHED);
  });

  it("serves cached files through the app's own protocol inside the desktop app", () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {}, configurable: true });
    expect(toImageSrc(CACHED)).toBe("http://l8art.localhost/5/cover-36aebf0c3dee7a41.jpg");
    expect(toImageSrc("C:/Users/Ryu/AppData/Local/com.l8play.app/artwork/12/icon-e393fcc40f9d821a.ico")).toBe(
      "http://l8art.localhost/12/icon-e393fcc40f9d821a.ico",
    );
  });

  it("leaves paths outside the artwork cache alone", () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {}, configurable: true });
    expect(toImageSrc("C:\\Users\\Ryu\\Pictures\\cover.png")).toBe("C:\\Users\\Ryu\\Pictures\\cover.png");
  });
});
