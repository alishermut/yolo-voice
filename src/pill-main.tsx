import React from "react";
import ReactDOM from "react-dom/client";
import { Pill } from "./components/Pill";
import "./pill-styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Pill />
  </React.StrictMode>,
);
