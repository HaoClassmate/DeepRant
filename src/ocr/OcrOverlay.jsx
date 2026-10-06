import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// 游戏上方的悬浮窗：透明、鼠标穿透，显示"译文(原文)"，几秒后自动清空
export default function OcrOverlay() {
    const [data, setData] = useState(null);

    useEffect(() => {
        document.documentElement.style.background = 'transparent';
        document.body.style.background = 'transparent';
        invoke('ocr_last_overlay').then((p) => p && setData(p)).catch(() => {});
        const off = listen('ocr-overlay', (e) => setData(e.payload));
        return () => { off.then((f) => f()); };
    }, []);

    useEffect(() => {
        if (!data || data.state === 'pending') return;
        const t = setTimeout(() => setData((d) => (d && d.id === data.id ? null : d)), data.seconds * 1000);
        return () => clearTimeout(t);
    }, [data]);

    if (!data) return null;
    return (
        // 贴着窗口底部：窗口在聊天区域上方，结果紧挨着聊天框
        <div className="fixed inset-0 flex flex-col justify-end p-1 pointer-events-none">
            <div className="rounded-lg px-3 py-2 text-white" style={{ background: 'rgba(10,12,16,0.82)', fontFamily: 'system-ui, "Microsoft YaHei", sans-serif' }}>
                {data.state === 'pending' && <div className="text-sm text-zinc-400">{data.message}</div>}
                {data.state === 'error' && <div className="text-sm text-red-400">{data.message}</div>}
                {data.state === 'done' && data.items.map((it, i) => (
                    <div key={i} className="text-[15px] leading-snug py-0.5 break-words">
                        {it.name && <span className="text-sky-300 mr-1">{it.name}:</span>}
                        <span>{it.zh || it.text}</span>
                        {it.zh && <span className="text-zinc-400 text-[13px] ml-1">({it.text})</span>}
                    </div>
                ))}
            </div>
        </div>
    );
}
