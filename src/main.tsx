import "@/styles/globals.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "@/app/App";
import { bootstrap } from "@/app/bootstrap";

const container = document.getElementById("root");
if (!container) throw new Error("Root element #root is missing from index.html");

// Settings, theme and language are applied before the first render, so the UI never flashes
// the wrong theme or untranslated text.
void bootstrap().then(() => {
  createRoot(container).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
});
