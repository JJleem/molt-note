import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ErrorBoundary } from "./screens/ErrorBoundary";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {/* 화면별 경계(App.tsx)가 받지 못한 것 — 사이드바·헤더 자체가 죽는 경우 — 을 받는다.
        이것이 없으면 까만 화면이 되고, 그러면 진행 중인 녹음에 손이 닿지 않는다. */}
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
