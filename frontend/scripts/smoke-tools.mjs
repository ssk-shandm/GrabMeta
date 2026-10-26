// Default: verify real transfer then cancel. --full opts into full installation/upgrade tests.
// CONNECT proxy injects transport failures without changing TLS trust or system settings.
import assert from 'node:assert/strict'
import { spawn, execFileSync } from 'node:child_process'
import { once } from 'node:events'
import { createServer } from 'node:http'
import { connect, createServer as createTcpServer } from 'node:net'
import { createHash } from 'node:crypto'
import { createReadStream } from 'node:fs'
import { readFile, writeFile, mkdir, readdir, copyFile, cp, rm } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const full = process.argv.includes('--full')
const executable = resolve(process.argv.slice(2).find(arg => !arg.startsWith('--')) || join(root, 'frontend/src-tauri/target/release/grabmeta-install-test.exe'))
assert.equal(executable.endsWith('grabmeta-install-test.exe'), true, 'Only the isolated release test executable is allowed')
const artifact = join(root, '.grabmeta-install-tools-' + new Date().toISOString().replaceAll(/[:.]/g, '-'))
const appDir = join(artifact, '中文 空格 程序')
const settingsPath = join(process.env.APPDATA, 'com.ssk-shandm.grabmeta.install-test', 'settings.json')
const savedSettings = await readFile(settingsPath).catch(error => { if (error.code !== 'ENOENT') throw error; return null })
await mkdir(appDir, { recursive: true })
await copyFile(executable, join(appDir, 'grabmeta-install-test.exe'))
await copyFile(join(root, 'LICENSE'), join(appDir, 'LICENSE'))
await cp(join(root, 'licenses'), join(appDir, 'licenses'), { recursive: true })
const results = { mode: full ? 'full-install' : 'transfer-only', testedAt: new Date().toISOString(), artifact, checks: [], profiles: {}, pageErrors: [] }
const sockets = new Set()
let blocked = false, tunnels = 0, app, browser, page, progressTimer
const wait = ms => new Promise(resolve => setTimeout(resolve, ms))
const proxy = createServer((_request, response) => response.writeHead(502).end())
proxy.on('connect', (request, socket, head) => {
  sockets.add(socket); socket.on('error', () => {}); socket.once('close', () => sockets.delete(socket))
  const match = request.url.match(/^([a-zA-Z0-9.-]+):443$/)
  if (blocked || !match) { socket.end('HTTP/1.1 502 Bad Gateway\r\n\r\n'); return }
  tunnels++
  const upstream = connect(443, match[1])
  sockets.add(upstream); upstream.once('close', () => sockets.delete(upstream))
  upstream.on('error', () => socket.destroy())
  socket.once('close', () => upstream.destroy())
  upstream.once('connect', () => {
    if (blocked) { socket.destroy(); upstream.destroy(); return }
    socket.write('HTTP/1.1 200 Connection Established\r\n\r\n')
    if (head.length) upstream.write(head)
    socket.pipe(upstream); upstream.pipe(socket)
  })
})
await new Promise(resolve => proxy.listen(0, '127.0.0.1', resolve))
const listener = createTcpServer()
await new Promise(resolve => listener.listen(0, '127.0.0.1', resolve))
const port = listener.address().port
await new Promise(resolve => listener.close(resolve))
const invoke = (command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args })
async function hash(path) {
  const digest = createHash('sha256')
  for await (const chunk of createReadStream(path)) digest.update(chunk)
  return digest.digest('hex')
}
async function binSnapshot() {
  const files = ['yt-dlp.exe', 'ffmpeg.exe', 'ffprobe.exe', 'tool-profile.json', 'licenses/installed-tools.json']
  return Object.fromEntries(await Promise.all(files.map(async name => [name, await hash(join(appDir, 'bin', name))])))
}
async function cleanStaging() {
  assert.deepEqual((await readdir(appDir)).filter(name => name.startsWith('.grabmeta-')), [], 'Staging/backup was not cleaned')
}
function record(name) { results.checks.push(name); console.log('PASS', name) }
async function begin(profile) {
  await page.evaluate(() => { window.__toolProgress = [] })
  await page.locator(`input[name="tool-profile"][value="${profile}"]`).check()
  await page.locator('.tool-dialog .primary-action').click()
  await page.getByRole('button', { name: '取消安装', exact: true }).waitFor()
  await page.waitForFunction(() => !document.querySelector('.tool-dialog .install-error'))
}
async function transferring() {
  await page.waitForFunction(() => window.__toolProgress.some(event => event.downloaded >= 256 * 1024 && event.total > 1024 * 1024) || !!document.querySelector('.tool-dialog .install-error'), null, { timeout: 180000 })
  const alert = page.locator('.tool-dialog .install-error')
  assert.equal(await alert.count(), 0, await alert.count() ? await alert.textContent() : '')
}
async function settled() {
  await page.waitForFunction(() => !document.querySelector('.tool-dialog .primary-action')?.disabled, null, { timeout: 120000 })
  await cleanStaging()
}
async function openTools() {
  if (await page.locator('.guide-layer').count()) await page.getByRole('button', { name: '跳过教程', exact: true }).click()
  if (await page.locator('.tool-dialog[open]').count()) return
  await page.getByText('关于', { exact: true }).first().click()
  await page.locator('[data-guide="install-tools"]').click()
  await page.locator('.tool-dialog[open]').waitFor()
}
async function installed(profile) {
  await page.waitForFunction(() => document.querySelector('.tool-dialog .primary-action')?.disabled === false, null, { timeout: 1800000 })
  const alert = page.locator('.tool-dialog .install-error')
  const error = await alert.count() ? await alert.textContent() : ''
  assert.equal(error, '', error)
  await page.locator('.tool-dialog[open]').waitFor({ state: 'hidden' })
  const status = await invoke('get_tool_status')
  assert.equal(status.installed, true); assert.equal(status.profile, profile)
  assert.equal(resolve(status.binPath), join(appDir, 'bin'))
  assert.deepEqual(status.missing, [])
  const marker = JSON.parse(await readFile(join(appDir, 'bin/tool-profile.json'), 'utf8'))
  assert.deepEqual(marker, JSON.parse(await readFile(join(appDir, 'bin/licenses/installed-tools.json'), 'utf8')))
  assert.equal(await hash(join(appDir, 'bin/yt-dlp.exe')), marker.ytdlp.sha256)
  assert.ok(marker.ffmpeg.name.startsWith('ffmpeg-N-') || marker.ffmpeg.name.startsWith('ffmpeg-master-latest-'))
  assert.ok(marker.ffmpeg.name.endsWith(profile === 'basic' ? '-win64-lgpl.zip' : '-win64-gpl.zip'))
  const build = await readFile(join(appDir, 'bin/licenses/ffmpeg/buildconf-output.txt'), 'utf8')
  assert.equal(build.includes('--enable-gpl'), profile === 'full')
  assert.equal(build.includes('--enable-nonfree'), false)
  assert.ok((await readdir(join(appDir, 'bin/licenses/yt-dlp'))).some(name => /third.party/i.test(name)))
  await cleanStaging()
  results.profiles[profile] = { status, marker, installedHashes: await binSnapshot() }
  await page.screenshot({ path: join(artifact, profile + '.png') })
  record(`real ${profile} download, SHA-256/PE/version/license validation and commit`)
}
try {
  const proxyUrl = 'http://127.0.0.1:' + proxy.address().port
  app = spawn(join(appDir, 'grabmeta-install-test.exe'), [], {
    cwd: artifact, stdio: 'ignore', windowsHide: true,
    env: { ...process.env, HTTPS_PROXY: 'http://127.0.0.1:1', HTTP_PROXY: 'http://127.0.0.1:1', ALL_PROXY: 'http://127.0.0.1:1', NO_PROXY: 'localhost,127.0.0.1',
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=' + port,
      WEBVIEW2_USER_DATA_FOLDER: join(artifact, 'webview') },
  })
  for (let attempt = 0; attempt < 90; attempt++) {
    assert.equal(app.exitCode, null, 'App exited before WebView2 was ready')
    try { browser = await chromium.connectOverCDP('http://127.0.0.1:' + port); break } catch { await wait(500) }
  }
  assert.ok(browser, 'WebView2 CDP unavailable')
  page = browser.contexts()[0].pages()[0]
  await page.evaluate(() => localStorage.setItem('grabmeta.locale', 'zh-CN'))
  await page.reload()
  page.setDefaultTimeout(180000)
  page.on('pageerror', error => results.pageErrors.push(error.message))
  await page.locator('.tool-dialog[open]').waitFor()
  assert.equal((await invoke('get_tool_status')).installed, false)
  const updateRejection = await invoke('download_and_install_update', { url: 'https://github.com/unused/test-setup.exe', fileName: 'test-setup.exe' }).then(() => undefined, (error) => error)
  assert.equal(updateRejection?.code, 'update.isolatedBuild')
  await page.evaluate(async () => {
    window.__toolProgress = []; window.__toolHistory = []
    const handler = window.__TAURI_INTERNALS__.transformCallback(event => window.__toolProgress.push(event.payload))
    await window.__TAURI_INTERNALS__.invoke('plugin:event|listen', { event: 'tool-download-progress', target: { kind: 'Any' }, handler })
  })
  const settings = await invoke('get_settings')
  await invoke('save_settings', { settings: { ...settings, proxyEnabled: true, proxyUrl } })
  let lastProgress = ''
  progressTimer = setInterval(async () => {
    const last = await page.evaluate(() => window.__toolProgress.at(-1)).catch(() => null)
    if (!last) return
    const key = last.stage + ':' + Math.floor((last.percent || 0) / 10)
    if (key !== lastProgress) { lastProgress = key; console.log('DOWNLOAD', last.profile, last.stage, last.percent, Math.round(last.downloaded / 1024 / 1024), 'MiB') }
  }, 10000)
  await begin('basic'); await transferring()
  const duplicate = await invoke('download_tools', { profile: 'basic' }).then(() => undefined, (error) => error)
  assert.equal(duplicate?.code, 'tools.busy')
  await page.getByRole('button', { name: '取消安装', exact: true }).click()
  await settled()
  assert.match(await page.locator('.tool-dialog .install-error').textContent(), /取消/)
  assert.equal((await invoke('get_tool_status')).installed, false)
  record('cancel first real download, duplicate rejection, no partial tools or staging')
  if (!full) {
    await begin('full'); await transferring()
    await page.getByRole('button', { name: '取消安装', exact: true }).click(); await settled()
    assert.equal((await invoke('get_tool_status')).installed, false)
    assert.ok(tunnels > 0, 'Explicit local proxy was not used')
    assert.deepEqual(results.pageErrors, [])
    record('full profile receives real bytes through local proxy; cancelled without waiting for installation')
    results.status = 'passed'
  } else {
  await begin('basic'); await installed('basic')
  const basicSnapshot = await binSnapshot()
  await openTools(); await begin('full'); await transferring()
  await page.getByRole('button', { name: '取消安装', exact: true }).click(); await settled()
  assert.deepEqual(await binSnapshot(), basicSnapshot)
  record('cancel basic-to-full upgrade preserves all installed tools and records')
  await begin('full'); await transferring()
  assert.ok(tunnels > 0, 'App bypassed failure-injection proxy')
  blocked = true; for (const socket of sockets) socket.destroy()
  await settled()
  assert.ok(await page.locator('.tool-dialog .install-error').textContent())
  assert.deepEqual(await binSnapshot(), basicSnapshot)
  record('real TLS stream interruption preserves old tools, shows error and cleans staging')
  blocked = false
  await begin('full'); await installed('full')
  assert.notEqual(results.profiles.full.installedHashes['ffmpeg.exe'], basicSnapshot['ffmpeg.exe'])
  assert.deepEqual(results.pageErrors, [])
  record('retry after interruption upgrades basic to full without page errors')
  results.status = 'passed'
  }
} catch (error) {
  results.status = 'failed'; results.error = error.stack; process.exitCode = 1; console.error(error)
  if (page) await page.screenshot({ path: join(artifact, 'failure.png') }).catch(() => {})
} finally {
  clearInterval(progressTimer)
  if (page) {
    results.progress = await page.evaluate(() => window.__toolProgress).catch(() => [])
    results.history = await page.evaluate(() => window.__toolHistory).catch(() => [])
  }
  if (app && app.exitCode === null) {
    if (page) await invoke('cancel_tool_download').catch(() => {})
    execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `(Get-Process -Id ${app.pid}).CloseMainWindow() | Out-Null`], { windowsHide: true })
    for (let i = 0; i < 100 && app.exitCode === null; i++) await wait(100)
    if (app.exitCode === null) app.kill()
  }
  await browser?.close().catch(() => {})
  for (const socket of sockets) socket.destroy()
  await new Promise(resolve => proxy.close(resolve))
  if (savedSettings) await writeFile(settingsPath, savedSettings)
  else await rm(settingsPath, { force: true })
  results.proxyTunnels = tunnels
  await writeFile(join(artifact, 'result.json'), JSON.stringify(results, null, 2) + '\n')
  console.log('Evidence retained:', artifact)
}
