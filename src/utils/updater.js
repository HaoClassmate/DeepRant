import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { invoke } from '@tauri-apps/api/core';
import { showSuccess, showError } from './toast';

// 拉 latest.json 的超时：清单只有 1KB 左右，15 秒还拿不到说明网络不通，直接报错
const CHECK_TIMEOUT_MS = 15 * 1000;
// 下载安装包的超时（整次请求的总时长，不是空闲时长）：
// 安装包约 21MB，国内直连 GitHub 实测约 80KB/s 也要 4~5 分钟，给 10 分钟；
// 不设的话连接卡死时会无限等待，按钮一直停在"检查更新中"
const DOWNLOAD_TIMEOUT_MS = 10 * 60 * 1000;

// 检查更新，统一带上超时
const checkWithTimeout = () => check({ timeout: CHECK_TIMEOUT_MS });

// 下载并安装更新，装好后重启
// onProgress(percent)：可选，开始下载时以 0 调用一次，之后每个数据块都回调当前百分比
// （拿不到总大小时 percent 为 null），供界面显示下载状态
export const installUpdate = async (update, onProgress) => {
    let downloaded = 0;
    let contentLength = 0;
    let lastPercent = -10;
    await update.downloadAndInstall((progress) => {
        if (progress.event === 'Started') {
            contentLength = progress.data.contentLength || 0;
            showSuccess(`开始下载 v${update.version} ${(contentLength / 1024 / 1024).toFixed(2)}MB`);
            onProgress?.(contentLength ? 0 : null);
        } else if (progress.event === 'Progress') {
            downloaded += progress.data.chunkLength;
            const percent = contentLength ? Math.floor((downloaded / contentLength) * 100) : 0;
            onProgress?.(contentLength ? percent : null);
            // 每 10% 提示一次，免得刷屏
            if (percent - lastPercent >= 10) {
                lastPercent = percent;
                showSuccess(`下载进度: ${percent}%`);
            }
        } else if (progress.event === 'Finished') {
            showSuccess('下载完成，准备安装');
        }
    }, { timeout: DOWNLOAD_TIMEOUT_MS });
    showSuccess('更新已完成，即将重启应用');
    await relaunch();
};

// 启动时检查一次：有新版本就问要不要更新；没网、没发布过等情况都静默忽略
export const checkUpdateOnStartup = async () => {
    try {
        const update = await checkWithTimeout();
        if (!update) return;
        const notes = update.body ? `\n\n${update.body}` : '';
        if (window.confirm(`发现新版本 v${update.version}（当前 v${update.currentVersion}），现在更新吗？${notes}`)) {
            await installUpdate(update);
        }
    } catch (error) {
        console.error('启动时检查更新失败:', error);
    }
};

// 开机自启时主窗口藏在托盘里，确认框弹在隐藏窗口上看不见，
// 所以等用户第一次打开主窗口（拿到焦点）再检查；普通启动照常检查
export const scheduleStartupUpdateCheck = async () => {
    let fromAutostart = false;
    try {
        fromAutostart = await invoke('launched_by_autostart');
    } catch (error) {
        console.error('读取启动方式失败:', error);
    }
    if (fromAutostart && !document.hasFocus()) {
        window.addEventListener('focus', () => checkUpdateOnStartup(), { once: true });
    } else {
        checkUpdateOnStartup();
    }
};

// 手动检查（关于页的按钮）
// onProgress：可选，透传给 installUpdate，调用方据此把按钮切到"下载中"并显示百分比
export const checkUpdateManually = async (onProgress) => {
    try {
        const update = await checkWithTimeout();
        if (update) {
            await installUpdate(update, onProgress);
            return 'installed';
        }
        showSuccess('当前已是最新版本');
        return 'idle';
    } catch (error) {
        console.error('Update error:', error);
        showError(`更新失败: ${error.message || error}`);
        return 'error';
    }
};
