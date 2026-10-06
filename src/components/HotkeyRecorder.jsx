import { useEffect, useRef, useState } from 'react';

const MODIFIERS = ['Control', 'Alt', 'Shift', 'Meta'];
const isModifier = (code) => MODIFIERS.some((m) => code.includes(m));
const label = (code) => {
    if (isModifier(code)) {
        const m = code.replace('Left', '').replace('Right', '');
        return { Control: 'Ctrl', Alt: 'Alt', Shift: '⇧', Meta: 'Win' }[m] || m;
    }
    return code.replace('Key', '').replace('Digit', '');
};

// 点一下开始录：按下组合键，松开任意键结束，onChange 收到按键数组（e.code，修饰键在前）
export default function HotkeyRecorder({ value, onChange }) {
    const [recording, setRecording] = useState(false);
    const [keys, setKeys] = useState([]);
    const pressed = useRef([]);

    useEffect(() => {
        if (!recording) return;
        const down = (e) => {
            e.preventDefault();
            if (e.code === 'Escape') { setRecording(false); return; }
            if (!pressed.current.includes(e.code)) {
                // 修饰键排前面，主键放最后（Rust 端按"最后一个是主键"解析）
                const all = [...pressed.current, e.code];
                const next = [...all.filter(isModifier), ...all.filter((k) => !isModifier(k))];
                pressed.current = next;
                setKeys(next);
            }
        };
        const up = (e) => {
            e.preventDefault();
            const result = pressed.current;
            pressed.current = [];
            setKeys([]);
            setRecording(false);
            if (result.length) onChange(result);
        };
        window.addEventListener('keydown', down);
        window.addEventListener('keyup', up);
        return () => { window.removeEventListener('keydown', down); window.removeEventListener('keyup', up); };
    }, [recording, onChange]);

    return (
        <button
            type="button"
            onClick={() => { pressed.current = []; setKeys([]); setRecording(true); }}
            className={`min-w-[120px] px-3 py-1.5 rounded-lg border text-sm font-semibold transition-colors ${recording ? 'border-sky-400 bg-sky-50 text-sky-700' : 'border-zinc-200 bg-zinc-50 text-zinc-800 hover:border-zinc-400'}`}
            title="点击后按下新的组合键，Esc 取消"
        >
            {recording ? (keys.length ? keys.map(label).join(' + ') : '请按下组合键…') : value || '未设置'}
        </button>
    );
}
