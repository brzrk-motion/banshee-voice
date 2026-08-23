import React from "react";
import ReactDOM from "react-dom/client";
import App from "./app/App";
import { HudApp } from "./hud/HudApp";
import "./styles/index.css";
import "./styles/hud.css";

const params = new URLSearchParams(window.location.search);
const Root = params.get("view") === "hud" ? HudApp : App;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
