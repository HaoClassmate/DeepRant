import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { showSuccess, showError } from './toast';

// 下载并安装更新，装好后重启
export const installUpdate = async (update) => {
    let downloaded = 0;
    let contentLength = 0;
    let lastPercent = -10;
    await update.downloadAndInstall((progress) => {
        if (progress.event === 'Started') {
            contentLength = progress.data.contentLength || 0;
            showSuccess(`开始下载 v${update.version} ${(contentLength / 1024 / 1024).toFixed(2)}MB`);
        } else if (progress.event === 'Progress') {
            downloaded += progress.data.chunkLength;
            const percent = contentLength ? Math.floor((downloaded / contentLength) * 100) : 0;
            // 每 10% 提示一次，免得刷屏
            if (percent - lastPercent >= 10) {
                lastPercent = percent;
                showSuccess(`下载进度: ${percent}%`);
            }
        } else if (progress.event === 'Finished') {
            showSuccess('下载完成，准备安装');
        }
    });
    showSuccess('更新已完成，即将重启应用');
    await relaunch();
};

// 启动时检查一次：有新版本就问要不要更新；没网、没发布过等情况都静默忽略
export const checkUpdateOnStartup = async () => {
    try {
        const update = await check();
        if (!update) return;
        const notes = update.body ? `\n\n${update.body}` : '';
        if (window.confirm(`发现新版本 v${update.version}（当前 v${update.currentVersion}），现在更新吗？${notes}`)) {
            await installUpdate(update);
        }
    } catch (error) {
        console.error('启动时检查更新失败:', error);
    }
};

// 手动检查（关于页的按钮）
export const checkUpdateManually = async () => {
    try {
        const update = await check();
        if (update) {
            await installUpdate(update);
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
