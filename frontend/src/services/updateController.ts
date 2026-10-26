import { reactive } from 'vue'
import type { CodedError } from '@/types/desktop'

export type UpdatePhase = 'idle' | 'checking' | 'latest' | 'waiting' | 'downloading' | 'installing' | 'error'
export interface UpdateInfo {
  version: string
  available: boolean
  releaseUrl?: string | null
  releaseNotes?: string | null
  installerUrl?: string | null
  installerName?: string | null
  installerSha256?: string | null
}
export type NoticeParams = Record<string, string | number>
export interface UpdateDependencies {
  isDesktop: () => boolean
  activeTasks: () => boolean
  check: () => Promise<UpdateInfo>
  install: (update: UpdateInfo) => Promise<void>
  readPreference: () => string | null
  savePreference: (value: string) => void
  translate?: (key: string, params: NoticeParams) => string
}
export interface NoticeError extends Error {
  key: string
  params: NoticeParams
}
export const UPDATE_STORAGE_KEY = 'grabmeta.auto-update-check'
// Pre-2.3.3 builds stored the preference under the old project name. Read it as a
// fallback so renaming the key does not silently re-enable automatic updates.
export const LEGACY_UPDATE_STORAGE_KEY = 'ytdlp.auto-update-check'

function isNoticeError(error: unknown): error is NoticeError {
  return error instanceof Error && typeof (error as Partial<NoticeError>).key === 'string'
}

function isCodedError(error: unknown): error is CodedError {
  return typeof error === 'object' && error !== null && typeof (error as Partial<CodedError>).code === 'string'
}

export function createUpdateController(deps: UpdateDependencies) {
  const translate = deps.translate ?? ((key: string) => key)
  const state = reactive({
    autoCheck: true,
    phase: 'idle' as UpdatePhase,
    notice: { key: '', params: {} as NoticeParams, raw: '' },
    preferenceErrorKey: '',
    version: '',
    progress: null as number | null,
    get message(): string {
      return this.notice.key ? translate(this.notice.key, this.notice.params) : this.notice.raw
    },
    get preferenceError(): string {
      return this.preferenceErrorKey ? translate(this.preferenceErrorKey, {}) : ''
    },
  })
  let busy = false
  let initialized = false
  let pendingUpdate: UpdateInfo | null = null
  let automatic = false
  let latestMessageTimer: ReturnType<typeof setTimeout> | undefined

  function notify(key: string, params: NoticeParams = {}) {
    state.notice = { key, params, raw: '' }
  }

  function notifyRaw(text: string) {
    state.notice = { key: '', params: {}, raw: text }
  }

  function fail(error: unknown) {
    state.phase = 'error'
    if (isNoticeError(error)) notify(error.key, error.params)
    else if (isCodedError(error)) notify('codes.' + error.code, { detail: error.detail ?? '' })
    else notifyRaw(String(error))
  }

  function clearLatestMessageTimer() {
    if (latestMessageTimer) {
      clearTimeout(latestMessageTimer)
      latestMessageTimer = undefined
    }
  }

  function initializePreference() {
    if (initialized) return
    initialized = true
    try {
      const saved = deps.readPreference()
      state.autoCheck = saved === null ? true : saved === 'true'
    } catch {
      state.preferenceErrorKey = 'updates.prefReadError'
    }
  }

  function savePreference() {
    state.preferenceErrorKey = ''
    try {
      deps.savePreference(String(state.autoCheck))
    } catch {
      state.preferenceErrorKey = 'updates.prefSaveError'
    }
    if (!state.autoCheck && automatic && pendingUpdate) {
      pendingUpdate = null
      state.phase = 'idle'
      notify('updates.autoDisabled')
    }
  }

  async function installPending() {
    const update = pendingUpdate
    if (!update || busy) return
    if (automatic && !state.autoCheck) return
    if (deps.activeTasks()) {
      state.phase = 'waiting'
      notify('updates.waitingForTasks', { version: state.version })
      return
    }
    busy = true
    pendingUpdate = null
    state.progress = null
    state.phase = 'downloading'
    if (update.installerName) notify('updates.downloadingInstaller', { version: state.version })
    else notify('updates.noInstaller', { version: state.version })
    try {
      await deps.install(update)
      state.phase = 'installing'
      notify('updates.installerStarted')
    } catch (error) {
      fail(error)
    } finally {
      busy = false
    }
  }

  async function checkAndInstall(source: 'manual' | 'automatic' = 'manual') {
    if (busy || pendingUpdate || state.phase === 'installing') return
    if (!deps.isDesktop()) {
      state.phase = 'error'
      notify('updates.browserPreview')
      return
    }
    automatic = source === 'automatic'
    clearLatestMessageTimer()
    busy = true
    state.phase = 'checking'
    state.progress = null
    notify('updates.checking')
    try {
      const update = await deps.check()
      state.version = update.version
      if (!update.available) {
        state.phase = 'latest'
        notify('updates.latest')
        latestMessageTimer = setTimeout(() => {
          if (state.phase === 'latest' && state.notice.key === 'updates.latest') {
            state.phase = 'idle'
            notifyRaw('')
          }
          latestMessageTimer = undefined
        }, 3000)
      } else if (!automatic || state.autoCheck) {
        pendingUpdate = update
      } else {
        state.phase = 'idle'
        notify('updates.autoDisabledSkipped')
      }
    } catch (error) {
      fail(error)
    } finally {
      busy = false
    }
    await installPending()
  }

  function progress(event: { downloaded: number; total?: number | null; percent?: number | null }) {
    state.phase = 'downloading'
    state.progress = event.percent ?? (event.total ? Math.min(100, Math.round(event.downloaded / event.total * 100)) : null)
    notify('updates.downloadingProgress')
  }

  function installing() {
    state.phase = 'installing'
    state.progress = 100
    notify('updates.installerStarted')
  }

  return { state, initializePreference, savePreference, checkAndInstall, installPending, progress, installing, fail }
}
