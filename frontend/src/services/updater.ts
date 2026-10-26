import { parseRelease, type ReleaseData } from './updateRelease'
import { toolManager } from './tools'
import { initializeDesktop } from './desktop'
import { isTauri, invoke } from '@tauri-apps/api/core'
import { APP_VERSION, RELEASES_URL } from '@/config/appInfo'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { watch } from 'vue'
import { useTaskStore } from '@/stores/taskStore'
import { createUpdateController, UPDATE_STORAGE_KEY, LEGACY_UPDATE_STORAGE_KEY, type UpdateInfo } from './updateController'
import { noticeError } from './updateRelease'
import { i18n } from '../i18n'
import { errorText } from '../i18n/codes'

const GITHUB_API_URL = 'https://api.github.com/repos/ssk-shandm/GrabMeta/releases/latest'

async function fetchReleaseInWebView(signal: AbortSignal): Promise<ReleaseData> {
  const response = await fetch(GITHUB_API_URL, {
    headers: { Accept: 'application/vnd.github+json' }, signal, cache: 'no-store',
  })
  if (!response.ok) {
    if (response.status === 403) throw noticeError('updates.rateLimited', 'GitHub API 请求受限，请稍后再试')
    if (response.status === 404) throw noticeError('updates.noRelease', 'GitHub 上还没有发布 Release')
    throw noticeError('updates.httpFailed', '更新检查失败（HTTP ' + response.status + '）', { status: response.status })
  }
  return response.json() as Promise<ReleaseData>
}

async function fetchLatestRelease(): Promise<UpdateInfo> {
  const controller = new AbortController()
  const timeout = window.setTimeout(() => controller.abort(), 20_000)
  try {
    // Desktop updates must use the same native network path as the installer.
    // This honors the persisted proxy/system routing and avoids a WebView/API
    // check succeeding while the native installer cannot reach the asset.
    const data = isTauri()
      ? await invoke<ReleaseData>('fetch_latest_release')
      : await fetchReleaseInWebView(controller.signal)
    return parseRelease(data, APP_VERSION, RELEASES_URL)
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') {
      throw noticeError('updates.timeout', '无法连接 GitHub 检查更新：请求超时')
    }
    const message = error instanceof Error ? error.message : String(error)
    if (message.includes('Failed to fetch') || message.includes('NetworkError')) {
      throw noticeError('updates.networkFailed', '无法连接 GitHub 检查更新：请检查网络或系统代理设置')
    }
    throw error
  } finally {
    window.clearTimeout(timeout)
  }
}

export const appUpdater = createUpdateController({
  translate: (key, params) => i18n.global.t(key, params),
  isDesktop: isTauri,
  activeTasks: () => toolManager.busy || useTaskStore().tasks.some((task) => task.status === 'running'),
  // Desktop checks and downloads share the native network path.
  check: fetchLatestRelease,
  install: (update) => {
    if (!update.installerUrl || !update.installerName) {
      return Promise.reject(noticeError('updates.noInstallerAsset', '该 Release 没有可自动安装的 Windows NSIS 安装包。'))
    }
    return invoke('download_and_install_update', {
      url: update.installerUrl,
      fileName: update.installerName,
      sha256: update.installerSha256,
    })
  },
  // Migrate the preference saved under the pre-2.3.3 key so the rename does not
  // silently re-enable automatic updates for existing users.
  readPreference: () => {
    const current = window.localStorage.getItem(UPDATE_STORAGE_KEY)
    if (current !== null) return current
    const legacy = window.localStorage.getItem(LEGACY_UPDATE_STORAGE_KEY)
    if (legacy !== null) window.localStorage.setItem(UPDATE_STORAGE_KEY, legacy)
    return legacy
  },
  savePreference: (value) => window.localStorage.setItem(UPDATE_STORAGE_KEY, value),
})

let unlistenProgress: UnlistenFn | undefined
let unlistenInstall: UnlistenFn | undefined
let stopWatch: (() => void) | undefined

export async function startAppUpdates() {
  appUpdater.initializePreference()
  if (!isTauri()) return
  try {
    // Load persisted settings before checking: the native updater needs the
    // user's proxy setting, and startup checks must not race initialization.
    await initializeDesktop()
    unlistenProgress = await listen<{ downloaded: number; total: number | null; percent: number | null }>(
      'update-download-progress',
      ({ payload }) => appUpdater.progress(payload),
    )
    unlistenInstall = await listen<string>('update-install-starting', () => appUpdater.installing())
    stopWatch = watch(
      () => toolManager.busy || useTaskStore().tasks.some((task) => task.status === 'running'),
      (active) => {
        if (!active) void appUpdater.installPending()
      },
    )
    if (appUpdater.state.autoCheck) await appUpdater.checkAndInstall('automatic')
  } catch (error) {
    appUpdater.fail(noticeError('updates.initFailed', errorText(error), { error: errorText(error) }))
  }
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    unlistenProgress?.()
    unlistenInstall?.()
    stopWatch?.()
  })
}
