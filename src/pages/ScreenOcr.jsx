import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { useStore } from '../components/StoreProvider';
import { showSuccess, showError } from '../utils/toast';

const DEFAULTS = {
    region: null,
    hotkey: { shortcut: 'Alt+Q' },
    select_hotkey: { shortcut: 'Alt+⇧+Q' },
    overlay_seconds: 12,
    push_enabled: false,
    push_url: 'https://dota.wanghaos.com/ocr',
    push_token: '',
};

const card = 'w-full bg-white rounded-2xl p-6 border border-zinc-200 shadow-[0_8px_30px_rgb(0,0,0,0.04)]';
const input = 'w-full px-3 py-2 rounded-lg border border-zinc-200 bg-zinc-50 text-sm focus:outline-none focus:border-zinc-400';
const button = 'px-4 py-2 rounded-lg text-sm font-medium transition-colors';

export default function ScreenOcr() {
    const { store, updateSettings } = useStore();
    const [screen, setScreen] = useState(DEFAULTS);

    // 区域是 Rust 在框选完后写进存储的，这里定时读最新值
    useEffect(() => {
        if (!store) return;
        let alive = true;
        const read = async () => {
            const s = await store.get('settings');
            if (alive && s) setScreen((cur) => ({ ...DEFAULTS, ...s.screen, ...(cur.dirty ? { ...cur, region: s.screen?.region ?? null } : {}) }));
        };
        read();
        const t = setInterval(read, 2000);
        return () => { alive = false; clearInterval(t); };
    }, [store]);

    // 保存前重新读一次，免得用旧数据覆盖掉刚框选的区域
    const save = async (patch) => {
        const latest = (await store.get('settings'))?.screen || {};
        const next = { ...DEFAULTS, ...latest, ...patch };
        delete next.dirty;
        await updateSettings({ screen: next });
        setScreen(next);
    };

    const edit = (patch) => setScreen((cur) => ({ ...cur, ...patch, dirty: true }));

    const run = async (cmd, ok) => {
        try { await invoke(cmd); if (ok) showSuccess(ok); } catch (e) { showError(String(e)); }
    };

    const r = screen.region;
    return (
        <div className="h-full flex flex-col gap-6 p-6 overflow-auto">
            <motion.div className={card} initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }}>
                <h1 className="text-2xl font-bold text-zinc-900 mb-2">截图翻译</h1>
                <p className="text-sm text-zinc-500 mb-5">看不懂队友在说什么时，截取游戏聊天框，识别文字后翻成中文，显示在聊天框上方。仅支持 Windows。</p>
                <ol className="text-sm text-zinc-700 space-y-2 mb-5 list-decimal list-inside">
                    <li>按 <b>{screen.select_hotkey?.shortcut}</b>（或点下面的按钮）框选游戏的聊天区域，只需要做一次。</li>
                    <li>游戏里按 <b>{screen.hotkey?.shortcut}</b>，翻译结果会出现在聊天框上方，{screen.overlay_seconds} 秒后消失。</li>
                    <li>识别用的是 Windows 自带 OCR，需要系统装有英语语言包（设置 → 时间和语言 → 语言）。</li>
                </ol>
                <div className="flex items-center gap-3 flex-wrap">
                    <span className="text-sm text-zinc-600">
                        聊天区域：{r ? `${r.width}×${r.height}，位置 (${r.x}, ${r.y})` : '还没有框选'}
                    </span>
                    <button className={`${button} bg-zinc-900 text-white hover:bg-zinc-700`} onClick={() => run('ocr_select_region')}>
                        {r ? '重新框选' : '框选聊天区域'}
                    </button>
                    <button className={`${button} bg-zinc-100 text-zinc-800 hover:bg-zinc-200 disabled:opacity-40`} disabled={!r} onClick={() => run('ocr_translate_now')}>
                        现在翻译一次
                    </button>
                </div>
            </motion.div>

            <motion.div className={card} initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.05 }}>
                <h2 className="text-lg font-semibold text-zinc-900 mb-4">显示</h2>
                <label className="flex items-center gap-3 text-sm text-zinc-700">
                    悬浮窗显示
                    <input type="number" min={3} max={120} className={`${input} w-24`} value={screen.overlay_seconds}
                        onChange={(e) => edit({ overlay_seconds: Number(e.target.value) || 12 })}
                        onBlur={() => save({ overlay_seconds: Math.min(120, Math.max(3, screen.overlay_seconds)) })} />
                    秒
                </label>
            </motion.div>

            <motion.div className={card} initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.1 }}>
                <div className="flex items-center justify-between mb-2">
                    <h2 className="text-lg font-semibold text-zinc-900">推送到网页</h2>
                    <label className="flex items-center gap-2 text-sm text-zinc-700 cursor-pointer">
                        <input type="checkbox" checked={!!screen.push_enabled} onChange={(e) => save({ push_enabled: e.target.checked })} />
                        开启
                    </label>
                </div>
                <p className="text-sm text-zinc-500 mb-4">翻译结果同时发到聊天翻译网页，另一台电脑或手机上也能看到；组队、私聊这些游戏数据推送收不到的聊天，也能靠截图补上。</p>
                <div className="grid gap-3">
                    <label className="text-sm text-zinc-700">
                        地址
                        <input className={`${input} mt-1`} value={screen.push_url}
                            onChange={(e) => edit({ push_url: e.target.value })} onBlur={() => save({ push_url: screen.push_url.trim() })} />
                    </label>
                    <label className="text-sm text-zinc-700">
                        口令（网页地址里 k= 后面那串）
                        <input className={`${input} mt-1`} type="password" value={screen.push_token}
                            onChange={(e) => edit({ push_token: e.target.value })} onBlur={() => save({ push_token: screen.push_token.trim() })} />
                    </label>
                </div>
            </motion.div>
        </div>
    );
}
