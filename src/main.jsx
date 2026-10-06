import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import RegionSelector from "./ocr/RegionSelector";
import OcrOverlay from "./ocr/OcrOverlay";
import "./index.css";

// 截图翻译的两个窗口和主窗口共用同一个页面，靠 Rust 注入的 __DEEPRANT_VIEW 区分
const views = { selector: RegionSelector, overlay: OcrOverlay };
const View = views[window.__DEEPRANT_VIEW] || App;

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <View />
  </React.StrictMode>,
);
