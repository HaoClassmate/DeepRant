import { useEffect, useState } from "react";
import Layout from "./components/Layout";
import { checkUpdateOnStartup } from "./utils/updater";
import Home from "./pages/home";
import Translate from "./pages/Translate";
import User from "./pages/User";
import Settings from "./pages/Settings";
import About from "./pages/About";
import Mana from "./pages/Mana";
import Phrases from "./pages/Phrases";
import ScreenOcr from "./pages/ScreenOcr";

const pages = {
  home: Home,
  translate: Translate,
  user: User,
  settings: Settings,
  about: About,
  mana: Mana,
  phrases: Phrases,
  screenOcr: ScreenOcr
};

// 开发模式下 StrictMode 会让 effect 跑两次，用它保证只检查一次
let updateChecked = false;

function App() {
  const [activeItem, setActiveItem] = useState("home");

  useEffect(() => {
    if (updateChecked || import.meta.env.DEV) return;
    updateChecked = true;
    // 稍等一下，别和启动时的其他初始化挤在一起
    const timer = setTimeout(checkUpdateOnStartup, 3000);
    return () => clearTimeout(timer);
  }, []);
  const CurrentPage = pages[activeItem];

  return (
    <div className="min-h-screen bg-white text-zinc-900">
      <Layout activeItem={activeItem} setActiveItem={setActiveItem}>
        <CurrentPage />
      </Layout>
    </div>
  );
}

export default App;
