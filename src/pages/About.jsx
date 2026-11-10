import { motion } from 'framer-motion';
import { GamingPad, Globe, Translate, Github, AT } from '../icons';
import DeveloperNote from '../components/DeveloperNote';
import { useStore } from '../components/StoreProvider';
import { checkUpdateManually } from '../utils/updater';
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function About() {
    const [updateStatus, setUpdateStatus] = useState('idle');
    const [currentVersion, setCurrentVersion] = useState('');
    const [autostart, setAutostart] = useState(false);
    const [autostartBusy, setAutostartBusy] = useState(true);

    useEffect(() => {
        invoke('get_version')
            .then(version => {
                setCurrentVersion(version);
            })
            .catch(error => {
                console.error('获取版本号失败:', error);
            });
        invoke('get_autostart')
            .then(enabled => setAutostart(enabled))
            .catch(error => console.error('读取开机自启动状态失败:', error))
            .finally(() => setAutostartBusy(false));
    }, []);

    const toggleAutostart = async () => {
        const next = !autostart;
        setAutostartBusy(true);
        try {
            const enabled = await invoke('set_autostart', { enabled: next });
            setAutostart(enabled);
            showSuccess(enabled ? '已开启开机自启动' : '已关闭开机自启动');
        } catch (error) {
            console.error('设置开机自启动失败:', error);
            showError(typeof error === 'string' ? error : '设置开机自启动失败');
        } finally {
            setAutostartBusy(false);
        }
    };

    const checkUpdate = async () => {
        setUpdateStatus('checking');
        setUpdateStatus(await checkUpdateManually());
    };

    return (
        <div className="h-full flex flex-col gap-6">
            {/* 头部介绍区域 */}
            <motion.div
                className="w-full bg-white dark:bg-zinc-900 rounded-2xl p-6 border border-zinc-200 dark:border-zinc-800 shadow-[0_8px_30px_rgb(0,0,0,0.04)] dark:shadow-[0_8px_30px_rgb(0,0,0,0.1)] backdrop-blur-sm"
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
            >
                <div className="flex items-center justify-between mb-4">
                    <h1 className="text-2xl font-bold text-zinc-900 dark:text-white">关于 DeepRant</h1>
                    <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-zinc-100 dark:bg-zinc-800">
                        <AT className="w-4 h-4 stroke-zinc-500" />
                        <span className="text-sm text-zinc-500">版本 {currentVersion}</span>
                        <button
                            onClick={checkUpdate}
                            disabled={updateStatus === 'checking' || updateStatus === 'downloading'}
                            className={`px-3 py-1 rounded-full text-xs flex items-center gap-1 transition-colors ${updateStatus === 'checking' || updateStatus === 'downloading'
                                ? 'bg-zinc-200 text-zinc-500 cursor-not-allowed'
                                : 'bg-blue-100 text-blue-600 hover:bg-blue-200'
                                }`}
                        >
                            {updateStatus === 'checking' && '检查更新中...'}
                            {updateStatus === 'downloading' && '下载更新中...'}
                            {updateStatus === 'idle' && '检查更新'}
                            {updateStatus === 'error' && '重试更新'}
                        </button>
                    </div>
                </div>
                <p className="text-zinc-600 dark:text-zinc-400">
                    DeepRant 是一款专为国际服游戏玩家打造的实时翻译工具，致力于打破语言壁垒，让全球玩家畅享跨语言交流的乐趣。
                </p>
                <div className="mt-4 pt-4 border-t border-zinc-100 dark:border-zinc-800 flex items-center justify-between">
                    <div>
                        <div className="text-sm font-medium text-zinc-900 dark:text-white">开机自启动</div>
                        <div className="text-xs text-zinc-400 mt-0.5">登录系统后自动在后台运行（只显示托盘图标），快捷键直接可用</div>
                    </div>
                    <button
                        type="button"
                        role="switch"
                        aria-checked={autostart}
                        aria-label="开机自启动"
                        onClick={toggleAutostart}
                        disabled={autostartBusy}
                        className={`relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors ${autostart ? 'bg-blue-500' : 'bg-zinc-300 dark:bg-zinc-700'} ${autostartBusy ? 'opacity-60 cursor-not-allowed' : 'cursor-pointer'}`}
                    >
                        <span
                            className={`inline-block h-5 w-5 transform rounded-full bg-white shadow transition-transform ${autostart ? 'translate-x-5' : 'translate-x-0.5'}`}
                        />
                    </button>
                </div>
            </motion.div>

            {/* 特性卡片网格 */}
            <div className="flex-1 grid grid-cols-1 md:grid-cols-3 gap-6">
                {/* 开发者说 */}
                <motion.div
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.1 }}
                >
                    <DeveloperNote />
                </motion.div>

                {/* 游戏场景优化 */}
                <motion.div
                    className="flex flex-col bg-white dark:bg-zinc-900 rounded-2xl p-6 border border-zinc-200 dark:border-zinc-800 shadow-[0_8px_30px_rgb(0,0,0,0.04)] dark:shadow-[0_8px_30px_rgb(0,0,0,0.1)] backdrop-blur-sm"
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.2 }}
                >
                    <div className="flex items-center gap-3 text-sm text-zinc-500">
                        <GamingPad className="w-6 h-6 stroke-zinc-500" />
                        游戏场景优化
                    </div>
                    <div className="mt-4 text-sm text-zinc-400">
                        针对不同游戏类型定制翻译策略，准确理解游戏术语和表达方式，让交流更加自然顺畅。
                    </div>
                </motion.div>

                {/* 多语言支持 */}
                <motion.div
                    className="flex flex-col bg-white dark:bg-zinc-900 rounded-2xl p-6 border border-zinc-200 dark:border-zinc-800 shadow-[0_8px_30px_rgb(0,0,0,0.04)] dark:shadow-[0_8px_30px_rgb(0,0,0,0.1)] backdrop-blur-sm"
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.3 }}
                >
                    <div className="flex items-center gap-3 text-sm text-zinc-500">
                        <Globe className="w-6 h-6 stroke-zinc-500" />
                        全球语言支持
                    </div>
                    <div className="mt-4 text-sm text-zinc-400">
                        支持多种语言之间的互译，让来自世界各地的玩家都能无障碍交流。
                    </div>
                </motion.div>
            </div>
        </div>
    );
} 