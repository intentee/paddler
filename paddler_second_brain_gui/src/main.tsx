import React from "react";
import { createRoot } from "react-dom/client";

function App() {
  return <h1>Paddler Second Brain</h1>;
}

const rootElement = document.getElementById("root") as HTMLElement;
const root = createRoot(rootElement);
root.render(<App />);
