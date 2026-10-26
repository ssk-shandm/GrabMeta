import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, readdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { createPinia, setActivePinia } from 'pinia'
import { useTaskStore } from '../src/stores/taskStore.ts'
import { useTerminalStore } from '../src/stores/terminalStore.ts'
import { useSettingsStore } from '../src/stores/settingsStore.ts'
import { useUrlStore } from '../src/stores/urlStore.ts'
import { createToolController } from '../src/services/toolController.ts'

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8')
test('task events update one task rather than append duplicate rows', () => {
  setActivePinia(createPinia())
  const store = useTaskStore()
  store.updateTask({ taskId: 1, kind: 'quick', status: 'running', message: { code: 'task.started' } })
  store.updateTask({ taskId: 1, kind: 'quick', status: 'success', message: { code: 'task.completed' } })
  assert.equal(store.tasks.length, 1)
  assert.equal(store.tasks[0].status, 'success')
})
test('task history is bounded without discarding active tasks', () => {
  setActivePinia(createPinia())
  const store = useTaskStore()
  store.updateTask({ taskId: 1, kind: 'quick', status: 'running', message: { code: 'task.started' } })
  for (let taskId = 2; taskId <= 250; taskId++)
    store.updateTask({ taskId, kind: 'quick', status: 'success', message: { code: 'task.completed' } })
  assert.equal(store.tasks.length, 201)
  assert.equal(store.tasks.find((task) => task.taskId === 1).status, 'running')
})
test('terminal preserves plain text and limits retained output', () => {
  setActivePinia(createPinia())
  const store = useTerminalStore()
  for (let i = 0; i < 2100; i++) store.addLine(String(i))
  store.addLine('<script>alert(1)</script>')
  assert.equal(store.state.output.length, 2000)
  assert.equal(store.state.output.at(-1), '<script>alert(1)</script>')
  assert.doesNotMatch(read('../src/pages/终端显示/components/MiniTerminal.vue'), /v-html/)
})
test('settings use string retries and can hydrate from native configuration', () => {
  setActivePinia(createPinia())
  const store = useSettingsStore()
  assert.equal(store.retryTimes, '10')
  assert.equal(store.concurrentFragments, 8)
  assert.equal(store.initialized, false)
  assert.equal(store.proxyEnabled, false)
  assert.equal(store.proxyUrl, 'http://127.0.0.1:7890')
  store.applySettings({ downloadPath: 'C:\\测试 下载', retryTimes: 'infinite' })
  assert.equal(store.retryTimes, 'infinite')
  assert.match(store.downloadPath, /测试/)
  assert.equal(store.concurrentFragments, 8)
  store.applySettings({ downloadPath: store.downloadPath, retryTimes: '3', concurrentFragments: 4 })
  assert.equal(store.concurrentFragments, 4)
  store.applySettings({ downloadPath: store.downloadPath, retryTimes: '3', concurrentFragments: 4, proxyEnabled: true, proxyUrl: 'socks5://127.0.0.1:1080' })
  assert.equal(store.proxyEnabled, true)
  assert.equal(store.proxyUrl, 'socks5://127.0.0.1:1080')
})
test('tool setup opens on first run, blocks active tasks, and reports failed installs', async () => {
  let installs = 0
  let hasActiveTasks = true
  const controller = createToolController({
    isDesktop: () => true,
    activeTasks: () => hasActiveTasks,
    getStatus: async () => ({ installed: false, profile: null, binPath: 'bin', ytdlpVersion: null, ffmpegVersion: null, ffprobeVersion: null, missing: [], error: null }),
    install: async () => { installs++; throw new Error('网络失败') },
    cancel: async () => {},
  })
  await controller.refresh(true)
  assert.equal(controller.state.visible, true)
  await controller.install('full')
  assert.equal(installs, 0)
  assert.equal(controller.state.error.key, 'tools.errors.activeTasks')
  hasActiveTasks = false
  await controller.install('full')
  assert.equal(installs, 1)
  assert.match(controller.state.error, /网络失败/)
  assert.equal(controller.state.message.key, 'tools.status.failed')
})
test('retry waits for the cancellation IPC and cannot receive a late cancel', async () => {
  let rejectInstall, resolveCancel, installs = 0
  const status = { installed: true, profile: 'basic', binPath: 'bin', missing: [], error: null }
  const controller = createToolController({
    isDesktop: () => true, activeTasks: () => false, getStatus: async () => status,
    install: () => { installs++; return installs === 1 ? new Promise((_resolve, reject) => { rejectInstall = reject }) : Promise.resolve(status) },
    cancel: () => new Promise(resolve => { resolveCancel = resolve }),
  })
  const first = controller.install('basic')
  const cancel = controller.cancel()
  await Promise.resolve()
  rejectInstall(new Error('cancelled'))
  await Promise.resolve()
  await Promise.resolve()
  assert.equal(controller.state.busy, true)
  await controller.install('basic')
  assert.equal(installs, 1)
  resolveCancel()
  await Promise.all([first, cancel])
  assert.equal(controller.state.busy, false)
  await controller.install('basic')
  assert.equal(installs, 2)
  assert.equal(controller.state.error, '')
  assert.equal(controller.state.status.installed, true)
})
test('failed cancellation is handled and does not wedge tool installation', async () => {
  let finish
  const status = { installed: true, profile: 'basic', binPath: 'bin', missing: [], error: null }
  const controller = createToolController({
    isDesktop: () => true, activeTasks: () => false, getStatus: async () => status,
    install: () => new Promise(resolve => { finish = resolve }),
    cancel: () => { throw new Error('cancel IPC unavailable') },
  })
  const pending = controller.install()
  await controller.cancel()
  assert.match(controller.state.error, /IPC unavailable/)
  assert.equal(controller.state.cancelling, false)
  finish(status)
  await pending
  assert.equal(controller.state.busy, false)
})
test('tool transfer progress shows measured speed, ETA, and resets for a new asset', async (t) => {
  let now = 1000
  t.mock.method(Date, 'now', () => now)
  let finish
  const status = { installed: true, profile: 'basic', binPath: 'bin', ytdlpVersion: '1', ffmpegVersion: '1', ffprobeVersion: '1', missing: [], error: null }
  const controller = createToolController({
    isDesktop: () => true, activeTasks: () => false,
    getStatus: async () => status,
    install: () => new Promise(resolve => { finish = resolve }),
    cancel: async () => {},
  })
  const pending = controller.install('basic')
  const payload = { profile: 'basic', stage: 'tools.stage.download', downloaded: 0, total: 4 * 1024 * 1024, percent: 0 }
  controller.progress(payload)
  now += 1000
  controller.progress({ ...payload, downloaded: 1024 * 1024, percent: 25 })
  assert.equal(controller.state.speed, '1.00 MiB/s')
  assert.deepEqual(controller.state.remaining, { key: 'tools.remaining.seconds', values: { n: 3 } })
  assert.deepEqual(controller.state.message.values, { stage: { key: 'codes.tools.stage.download' }, percent: 25, downloaded: '1.0', total: '4.0' })
  controller.progress({ ...payload, profile: 'full', stage: 'tools.stage.extract' })
  assert.equal(controller.state.stage, 'tools.stage.download')
  controller.progress({ ...payload, stage: 'tools.stage.verify' })
  assert.equal(controller.state.speed, '')
  assert.equal(controller.state.remaining, '')
  finish(status)
  await pending
  controller.progress({ ...payload, stage: 'ignored-after-install' })
  assert.notEqual(controller.state.stage, 'ignored-after-install')
})
test('versions, NSIS configuration, resources and capabilities agree', () => {
  const config = JSON.parse(read('../src-tauri/tauri.conf.json'))
  const pkg = JSON.parse(read('../package.json'))
  const cargo = read('../src-tauri/Cargo.toml')
  assert.equal(config.version, pkg.version)
  assert.match(cargo, new RegExp('version = "' + pkg.version.replaceAll('.', '\\.') + '"'))
  assert.deepEqual(config.bundle.targets, ['nsis'])
  assert.equal(config.bundle.windows.nsis.installMode, 'currentUser')
  assert.equal(config.bundle.resources['../../bin/yt-dlp.exe'], undefined)
  assert.equal(config.bundle.resources['../../bin/ffmpeg.exe'], undefined)
  assert.equal(config.bundle.resources['../../bin/ffprobe.exe'], undefined)
  assert.equal(config.bundle.resources['../../licenses'], 'licenses')
  assert.match(config.app.security.csp, /object-src 'none'/)
  const capability = JSON.parse(read('../src-tauri/capabilities/main.json'))
  assert.deepEqual(capability.permissions.slice(0, 2), ['core:default', 'dialog:allow-open'])
  assert.deepEqual(capability.permissions[2], {
    identifier: 'opener:allow-open-url',
    allow: [
      { url: 'https://github.com/ssk-shandm/GrabMeta' },
      { url: 'https://github.com/ssk-shandm/GrabMeta/releases' },
      { url: 'https://github.com/ssk-shandm/GrabMeta/blob/main/LICENSE' },
      { url: 'https://github.com/yt-dlp/yt-dlp' },
      { url: 'https://ffmpeg.org/' },
      { url: 'https://github.com/BtbN/FFmpeg-Builds' },
    ],
  })
  assert.equal(capability.permissions.length, 3)
})
test('each frontend invoke has a registered Rust command, no Eel entry remains', () => {
  const service = read('../src/services/desktop.ts')
  const rust = read('../src-tauri/src/lib.rs')
  for (const match of service.matchAll(/invoke(?:<[^>]+>)?\('([^']+)'/g)) assert.ok(rust.includes(match[1]), match[1])
  assert.doesNotMatch(read('../src/main.ts') + read('../index.html'), /eel/)
  assert.ok(readdirSync(fileURLToPath(new URL('../../docs/', import.meta.url))).includes('CHANGELOG.md'))
})

test('thumbnail starts empty and resets without restoring a sample image', () => {
  setActivePinia(createPinia())
  const store = useUrlStore()
  assert.equal(store.thumbnailUrl, '')
  store.setThumbnailUrl('https://example.com/cover.jpg')
  assert.equal(store.thumbnailUrl, 'https://example.com/cover.jpg')
  store.resetThumbnail()
  assert.equal(store.thumbnailUrl, '')
})

test('Windows icon contains small and large sizes and all configured PNG icons are square', () => {
  const config = JSON.parse(read('../src-tauri/tauri.conf.json'))
  const ico = readFileSync(new URL('../src-tauri/icons/icon.ico', import.meta.url))
  assert.equal(ico.readUInt16LE(0), 0)
  assert.equal(ico.readUInt16LE(2), 1)
  const sizes = []
  for (let index = 0; index < ico.readUInt16LE(4); index++) {
    const offset = 6 + index * 16
    const width = ico[offset] || 256
    const height = ico[offset + 1] || 256
    assert.equal(width, height)
    sizes.push(width)
    assert.ok(ico.readUInt32LE(offset + 12) + ico.readUInt32LE(offset + 8) <= ico.length)
  }
  for (const size of [16, 32, 48, 256]) assert.ok(sizes.includes(size), `Missing ${size}px ICO entry`)
  for (const path of config.bundle.icon.filter((path) => path.endsWith('.png'))) {
    const png = readFileSync(new URL(`../src-tauri/${path}`, import.meta.url))
    assert.equal(png.subarray(0, 8).toString('hex'), '89504e470d0a1a0a')
    assert.equal(png.readUInt32BE(16), png.readUInt32BE(20))
  }
})
