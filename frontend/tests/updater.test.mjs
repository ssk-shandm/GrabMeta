import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { parseRelease, normalizeSha256, compareVersions } from '../src/services/updateRelease.ts'
import { createUpdateController, UPDATE_STORAGE_KEY, LEGACY_UPDATE_STORAGE_KEY } from '../src/services/updateController.ts'
function fixture(overrides = {}) {
  const calls = []
  const deps = {
    isDesktop: () => true, activeTasks: () => false,
    check: async () => { calls.push('check'); return { version: '1.2.0', available: true } },
    install: async () => { calls.push('install') },
    readPreference: () => 'true', savePreference: (value) => calls.push(value),
    ...overrides,
  }
  const controller = createUpdateController(deps)
  controller.initializePreference()
  return { controller, calls, deps }
}
test('manual update downloads/installs even when automatic updates are off', async () => {
  const { controller, calls } = fixture({ readPreference: () => null })
  await controller.checkAndInstall()
  assert.deepEqual(calls, ['check', 'install'])
  assert.equal(controller.state.phase, 'installing')
})
test('latest release does not download or install', async () => {
  const { controller, calls } = fixture({ check: async () => ({ available: false, version: '1.1.0' }) })
  await controller.checkAndInstall()
  assert.equal(controller.state.phase, 'latest')
  assert.deepEqual(calls, [])
})
test('parallel update clicks are ignored', async () => {
  let finish
  const { controller, calls } = fixture({ check: () => new Promise((resolve) => { finish = resolve }) })
  const operation = controller.checkAndInstall()
  await controller.checkAndInstall()
  finish({ version: '1.2.0', available: true })
  await operation
  assert.deepEqual(calls, ['install'])
})
test('update waits for active tasks, then installs once', async () => {
  let active = true
  const { controller, calls } = fixture({ activeTasks: () => active })
  await controller.checkAndInstall('automatic')
  assert.equal(controller.state.phase, 'waiting')
  assert.deepEqual(calls, ['check'])
  active = false
  await controller.installPending()
  await controller.installPending()
  assert.deepEqual(calls, ['check', 'install'])
})
test('disabling automatic update cancels a deferred automatic install', async () => {
  const { controller, calls, deps } = fixture({ activeTasks: () => true })
  await controller.checkAndInstall('automatic')
  controller.state.autoCheck = false
  controller.savePreference()
  deps.activeTasks = () => false
  await controller.installPending()
  assert.equal(controller.state.phase, 'idle')
  assert.deepEqual(calls, ['check', 'false'])
})
test('disabling auto check during network request prevents automatic installation', async () => {
  let finish
  const { controller, calls } = fixture({ check: () => new Promise((resolve) => { finish = resolve }) })
  const operation = controller.checkAndInstall('automatic')
  controller.state.autoCheck = false
  controller.savePreference()
  finish({ available: true, version: '1.2.0' })
  await operation
  assert.deepEqual(calls, ['false'])
})
test('errors allow retry and never imply successful installation', async () => {
  const { controller, deps } = fixture({ install: async () => { throw new Error('signature rejected') } })
  await controller.checkAndInstall()
  assert.equal(controller.state.phase, 'error')
  assert.match(controller.state.message, /signature rejected/)
  deps.install = async () => {}
  await controller.checkAndInstall()
  assert.equal(controller.state.phase, 'installing')
})
test('browser preview cannot initiate native checks or installs', async () => {
  const { controller, calls } = fixture({ isDesktop: () => false })
  await controller.checkAndInstall()
  assert.equal(controller.state.phase, 'error')
  assert.deepEqual(calls, [])
})
test('missing storage or content-length is handled gracefully', () => {
  const { controller } = fixture({ readPreference: () => { throw new Error('denied') } })
  assert.equal(controller.state.autoCheck, true)
  assert.ok(controller.state.preferenceError)
  controller.progress({ downloaded: 50, total: null })
  assert.equal(controller.state.progress, null)
  controller.progress({ downloaded: 125, total: 100 })
  assert.equal(controller.state.progress, 100)
})
test('release updates use the GitHub Release NSIS installer directly', () => {
  const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8')
  const config = JSON.parse(read('../src-tauri/tauri.conf.json'))
  assert.equal(config.bundle.createUpdaterArtifacts, undefined)
  assert.equal(config.plugins, undefined)
  const rust = read('../src-tauri/src/updater.rs')
  const frontend = read('../src/services/updater.ts')
  assert.match(frontend, /api\.github\.com\/repos\/ssk-shandm\/GrabMeta\/releases\/latest/)
  assert.match(frontend, /fetch\(GITHUB_API_URL/)
  assert.match(frontend, /isTauri\(\)[\s\S]*?invoke<ReleaseData>\('fetch_latest_release'\)/)
  assert.match(frontend, /await initializeDesktop\(\)/)
  assert.match(rust, /ends_with\("-setup\.exe"\)/)
  assert.match(rust, /download_and_install_update/)
  assert.match(rust, /update-download-progress/)
})

test('Rust update commands are registered without the Tauri updater plugin', () => {
  const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8')
  const rust = read('../src-tauri/src/lib.rs')
  for (const command of ['download_and_install_update', 'get_tool_info']) assert.match(rust, new RegExp('updater::' + command))
  assert.doesNotMatch(rust, /updater::check_app_update/)
  assert.doesNotMatch(rust, /tauri_plugin_updater/)
})


test('update downloader honors system trust/proxies without disabling TLS verification', () => {
  const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8')
  const cargo = read('../src-tauri/Cargo.toml')
  const rust = read('../src-tauri/src/updater.rs')
  assert.match(cargo, /"rustls-tls-native-roots"/)
  assert.match(cargo, /"system-proxy"/)
  assert.match(rust, /https_only\(true\)/)
  assert.match(rust, /connect_timeout/)
  assert.match(rust, /read_timeout/)
  assert.match(rust, /error\.source\(\)/)
  assert.match(rust, /update_network_error\(&error\)/)
  assert.doesNotMatch(rust, /update_network_error\("/)
  assert.doesNotMatch(rust, /danger_accept_invalid_(?:certs|hostnames)/)
})

test('release parser selects x64 NSIS and carries the GitHub SHA-256 into installation', async () => {
  const digest = 'AB'.repeat(32)
  const update = parseRelease({ tag_name: 'v2.0.2', assets: [
    { name: 'app_2.0.2_arm64-setup.exe', digest: 'bad' },
    null,
    { name: 'app_2.0.2_x64-setup.exe', browser_download_url: 'https://github.com/project/asset', digest: 'sha256:' + digest },
  ] }, '2.0.1', 'releases')
  assert.equal(update.installerName, 'app_2.0.2_x64-setup.exe')
  assert.equal(update.installerSha256, digest.toLowerCase())
  let installed
  const { controller } = fixture({ check: async () => update, install: async (value) => { installed = value } })
  await controller.checkAndInstall()
  assert.equal(installed.installerSha256, digest.toLowerCase())
})

test('missing or invalid digest never silently bypasses installer verification', async () => {
  for (const digest of [undefined, null, '', 'sha256:bad', 'sha512:' + 'a'.repeat(64), 'a'.repeat(64), 123]) {
    assert.throws(() => normalizeSha256(digest), /SHA-256/)
    const { controller, calls } = fixture({ check: async () => parseRelease({
      tag_name: 'v2.0.2', assets: [{ name: 'app_x64-setup.exe', digest }],
    }, '2.0.1', 'releases') })
    await controller.checkAndInstall()
    assert.equal(controller.state.phase, 'error')
    assert.ok(!calls.includes('install'))
  }
})

test('current releases need no digest; version comparison handles multi-digit parts', () => {
  assert.deepEqual(parseRelease({ tag_name: 'v2.0.1' }, '2.0.1', 'releases'), { available: false, version: '2.0.1' })
  assert.ok(compareVersions('2.10.0', '2.9.9') > 0)
  assert.throws(() => parseRelease({ tag_name: 'broken' }, '2.0.1', 'releases'), /版本号/)
  assert.throws(() => parseRelease({ tag_name: 'v2.0.2-rc.1' }, '2.0.1', 'releases'), /版本号/)
})

test('native updater validates before locking tasks and signals install only after launch', () => {
  const rust = readFileSync(new URL('../src-tauri/src/updater.rs', import.meta.url), 'utf8')
  assert.ok(rust.indexOf('let expected_sha256 = validate_update_sha256') < rust.indexOf('tasks.begin_update()?'))
  assert.ok(rust.indexOf('verify_update_sha256(hash, expected_sha256)?') < rust.indexOf('launch_installer(&target'))
  assert.ok(rust.indexOf('launch_installer(&target') < rust.indexOf('"update-install-starting"'))
  assert.match(rust, /PartialDownload\(partial\.clone\(\)\)/)
})

test('auto-update preference migrates from the pre-2.3.3 storage key', () => {
  const store = new Map()
  const saved = globalThis.window
  globalThis.window = {
    localStorage: {
      getItem: (key) => (store.has(key) ? store.get(key) : null),
      setItem: (key, value) => store.set(key, value),
    },
  }
  try {
    // Simulates an upgrade: only the old key exists, and it disabled auto updates.
    store.set(LEGACY_UPDATE_STORAGE_KEY, 'false')
    const controller = createUpdateController({
      isDesktop: () => true, activeTasks: () => false,
      check: async () => ({ version: '1.0.0', available: false }),
      install: async () => {},
      readPreference: () => {
        const current = window.localStorage.getItem(UPDATE_STORAGE_KEY)
        if (current !== null) return current
        const legacy = window.localStorage.getItem(LEGACY_UPDATE_STORAGE_KEY)
        if (legacy !== null) window.localStorage.setItem(UPDATE_STORAGE_KEY, legacy)
        return legacy
      },
      savePreference: (value) => window.localStorage.setItem(UPDATE_STORAGE_KEY, value),
    })
    controller.initializePreference()
    assert.equal(controller.state.autoCheck, false, 'a disabled preference must survive the key rename')
    assert.equal(store.get(UPDATE_STORAGE_KEY), 'false', 'the old value is copied to the new key')
  } finally {
    if (saved === undefined) delete globalThis.window
    else globalThis.window = saved
  }
})