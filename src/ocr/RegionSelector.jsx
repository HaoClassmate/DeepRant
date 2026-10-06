import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

// 盖住整个屏幕的半透明遮罩，拖一个框选中游戏的聊天区域；Esc 取消
export default function RegionSelector() {
    const [start, setStart] = useState(null);
    const [rect, setRect] = useState(null);
    const done = useRef(false);

    useEffect(() => {
        document.documentElement.style.background = 'transparent';
        document.body.style.background = 'transparent';
        const onKey = (e) => { if (e.key === 'Escape') invoke('ocr_cancel_select'); };
        window.addEventListener('keydown', onKey);
        return () => window.removeEventListener('keydown', onKey);
    }, []);

    const at = (e) => ({ x: e.clientX, y: e.clientY });
    const onDown = (e) => { if (e.button === 0) { setStart(at(e)); setRect(null); } };
    const onMove = (e) => {
        if (!start) return;
        const p = at(e);
        setRect({ x: Math.min(start.x, p.x), y: Math.min(start.y, p.y), width: Math.abs(p.x - start.x), height: Math.abs(p.y - start.y) });
    };
    const onUp = () => {
        setStart(null);
        if (!rect || rect.width < 20 || rect.height < 12 || done.current) return;
        done.current = true;
        invoke('ocr_region_selected', rect).catch((e) => { done.current = false; alert('保存区域失败：' + e); });
    };

    // 选框外面变暗，选框里面保持透明，看得清下面的聊天
    const shade = 'rgba(0,0,0,0.45)';
    return (
        <div
            className="fixed inset-0 select-none"
            style={{ cursor: 'crosshair', background: rect ? 'transparent' : shade }}
            onMouseDown={onDown}
            onMouseMove={onMove}
            onMouseUp={onUp}
        >
            {rect && (
                <div
                    className="absolute border-2 border-sky-400"
                    style={{ left: rect.x, top: rect.y, width: rect.width, height: rect.height, boxShadow: `0 0 0 9999px ${shade}` }}
                />
            )}
            <div className="absolute top-8 left-1/2 -translate-x-1/2 px-4 py-2 rounded-lg bg-black/75 text-white text-sm pointer-events-none">
                拖动鼠标框选游戏的聊天区域（只框聊天文字，越准越好）· Esc 取消
            </div>
        </div>
    );
}
