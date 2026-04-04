import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

if (import.meta.env.DEV) {
  import("react-devtools").then(() => {
    console.log("[DevTools] React DevTools 已连接");
  }).catch(() => {
    console.log("[DevTools] React DevTools 未启动，运行: npm run devtools");
  });
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
