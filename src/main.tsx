import "./styles.css";

import { createRoot } from "react-dom/client";

import App from "@/App";

const mount = document.querySelector<HTMLDivElement>("#root");
if (!mount) throw new Error("Agent Profiles management window is missing its mount point");

// No `StrictMode`: its double-invoked effects would start the sequential size
// walk twice on every list, and the whole point of that walk is that it is one
// directory at a time.
createRoot(mount).render(<App />);
