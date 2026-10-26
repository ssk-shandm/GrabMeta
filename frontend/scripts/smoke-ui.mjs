import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdtemp } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const port = 1421
const baseUrl = `http://127.0.0.1:${port}`
const artifactDir = await mkdtemp(join(tmpdir(), 'ytdlp-ui-'))
const browserPath =
  process.env.UI_BROWSER_PATH ||
  [
    join(process.env['PROGRAMFILES(X86)'] || '', 'Microsoft/Edge/Application/msedge.exe'),
    join(process.env.PROGRAMFILES || '', 'Google/Chrome/Application/chrome.exe'),
  ].find((path) => existsSync(path))
assert.ok(browserPath, 'Set UI_BROWSER_PATH to a Chromium browser executable')
const server = spawn(process.execPath, [join(root, 'node_modules/vite/bin/vite.js'), '--port', String(port)], {
  cwd: root,
  windowsHide: true,
  stdio: ['ignore', 'pipe', 'pipe'],
})
let serverOutput = ''
server.stdout.on('data', (data) => {
  serverOutput += data
})
server.stderr.on('data', (data) => {
  serverOutput += data
})
let browser
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
try {
  let ready = false
  for (let attempt = 0; attempt < 100; attempt++) {
    if (server.exitCode !== null) throw new Error(serverOutput)
    try {
      ready = (await fetch(baseUrl)).ok
    } catch {
      /* wait for the dev server */
    }
    if (ready) break
    await wait(200)
  }
  assert.ok(ready, 'Vite did not become ready: ' + serverOutput)
  browser = await chromium.launch({ executablePath: browserPath, headless: true })
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } })
  await context.addInitScript(() => localStorage.setItem('grabmeta.locale', 'zh-CN'))
  const page = await context.newPage()
  const errors = []
  page.on('pageerror', (error) => errors.push(error.message))
  page.on('console', (message) => {
    if (message.text().includes('[Vue warn]')) errors.push(message.text())
  })
  await page.goto(baseUrl)
  await page.locator('.empty-preview').waitFor()
  await page.waitForFunction(() => getComputedStyle(document.querySelector('.link-page')).opacity === '1')
  const notificationClose = page.locator('.t-notification .t-message__close')
  if (await notificationClose.count()) await notificationClose.first().evaluate((element) => element.click())
  assert.equal(await page.locator('.preview-img').count(), 0)
  assert.equal(await page.locator('.local-badge').count(), 0)
  assert.equal(await page.locator('.leftbar .brand').count(), 0)
  assert.equal(await page.locator('.leftbar .version-line').count(), 0)
  assert.equal(await page.getByRole('button', { name: '分析链接', exact: true }).isDisabled(), true)
  assert.equal(await page.getByRole('button', { name: '快速下载', exact: true }).isDisabled(), true)
  await page.screenshot({ path: join(artifactDir, 'home-1280.png'), fullPage: true })

  await page.getByRole('textbox', { name: '视频链接', exact: true }).fill('https://example.com/video')
  assert.equal(await page.getByRole('button', { name: '分析链接', exact: true }).isEnabled(), true)
  await page.locator('a.nav-item[title="关于"]').click()
  await page.locator('.about-page').waitFor()
  assert.equal(await page.locator('.about-notes').count(), 0)
  assert.equal(await page.locator('.about-footer').count(), 0)
  assert.equal(await page.locator('.about-page img').count(), 0)
  assert.deepEqual(await page.locator('.about-list dt').allTextContents(), ['版本号', '版权', '许可证', '项目仓库'])
  assert.equal(await page.locator('.about-section').count(), 3)
  assert.equal(await page.getByLabel('启用本地 VPN / 代理', { exact: true }).isChecked(), false)
  assert.equal(await page.getByLabel('本地代理地址', { exact: true }).isDisabled(), true)
  await page.getByLabel('启用本地 VPN / 代理', { exact: true }).check()
  await page.getByLabel('本地代理地址', { exact: true }).fill('socks5://127.0.0.1:1080')
  await page.getByRole('button', { name: '保存代理设置', exact: true }).click()
  await page.locator('.proxy-feedback').waitFor()
  assert.match(await page.locator('.proxy-feedback').innerText(), /浏览器预览不能保存/)
  assert.equal(await page.getByRole('button', { name: '保存代理设置', exact: true }).isEnabled(), true)
  assert.deepEqual(await page.locator('.about-section h2').allTextContents(), ['项目基本信息', '设置', '下载工具'])
  await page.locator('[data-guide="install-tools"]').click()
  const toolDialog = page.getByRole('dialog', { name: '准备你的下载工具', exact: true })
  await toolDialog.waitFor()
  assert.equal(await toolDialog.locator('input[value="basic"]').isChecked(), true)
  await toolDialog.locator('input[value="full"]').check()
  assert.equal(await toolDialog.locator('input[value="full"]').isChecked(), true)
  await toolDialog.getByRole('button', { name: '下载并安装', exact: true }).click()
  assert.match(await toolDialog.getByRole('alert').innerText(), /浏览器预览不能下载安装工具/)
  await toolDialog.getByRole('button', { name: '稍后安装', exact: true }).click()
  await toolDialog.waitFor({ state: 'hidden' })
  assert.equal(await page.locator('.switch-control input').isChecked(), true)
  await page.locator('.switch-control input').uncheck()
  assert.equal(await page.locator('.switch-control input').isChecked(), false)
  await page.locator('.switch-control input').check()
  assert.equal(await page.locator('.switch-control input').isChecked(), true)
  assert.equal(await page.evaluate(() => localStorage.getItem('grabmeta.auto-update-check')), 'true')
  await page.locator('.switch-control input').focus()
  await page.keyboard.press('Space')
  assert.equal(await page.locator('.switch-control input').isChecked(), false)
  await page.locator('.switch-control input').check()
  const savedSettingsPage = await page.context().newPage()
  await savedSettingsPage.goto(`${baseUrl}/#/about`)
  await savedSettingsPage.locator('.about-page').waitFor()
  assert.equal(await savedSettingsPage.locator('.switch-control input').isChecked(), true)
  await savedSettingsPage.close()
  await page.getByRole('button', { name: '检查并安装更新', exact: true }).click()
  await page.locator('.update-status').waitFor()
  assert.match(await page.locator('.update-status').innerText(), /浏览器界面预览不支持安装更新/)
  assert.equal(await page.getByRole('button', { name: '检查并安装更新', exact: true }).isEnabled(), true)
  assert.match(await page.locator('.about-list').innerText(), /版本号[\s\S]*版权[\s\S]*许可证[\s\S]*项目仓库/)
  assert.match(await page.locator('.about-list').innerText(), /Copyright © 2025 ssk-shandm/)
  await page.evaluate(() => {
    window.__openedUrls = []
    window.open = (url) => {
      window.__openedUrls.push(url)
      return null
    }
  })
  await page.locator('.repository-link').click()
  assert.deepEqual(await page.evaluate(() => window.__openedUrls), ['https://github.com/ssk-shandm/GrabMeta'])
  const denied = await page.evaluate(async () => {
    const { openExternalUrl } = await import('/src/services/openExternal.ts')
    try {
      await openExternalUrl('https://example.com/')
      return false
    } catch {
      return true
    }
  })
  assert.equal(denied, true)
  const nativeCall = await page.evaluate(async () => {
    const { openExternalUrl } = await import('/src/services/openExternal.ts')
    let call
    window.isTauri = true
    window.__TAURI_INTERNALS__ = {
      invoke: async (command, args) => {
        call = { command, args }
      },
    }
    try {
      await openExternalUrl('https://github.com/ssk-shandm/GrabMeta')
      return call
    } finally {
      delete window.isTauri
      delete window.__TAURI_INTERNALS__
    }
  })
  assert.equal(nativeCall.command, 'plugin:opener|open_url')
  assert.equal(nativeCall.args.url, 'https://github.com/ssk-shandm/GrabMeta')
  await page.locator('a.nav-item[title="图片链接"]').click()
  await page.locator('.link-page').waitFor()
  assert.equal(await page.locator('#video-url input').inputValue(), 'https://example.com/video')

  await page.evaluate(async () => {
    const { useUrlStore } = await import('/src/stores/urlStore.ts')
    const store = useUrlStore()
    store.analyzedUrl = store.currentUrl
    store.setThumbnailUrl(
      'data:image/svg+xml,' +
        encodeURIComponent(
          '<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180"><rect width="320" height="180" fill="#176bcc"/></svg>',
        ),
    )
  })
  await page.locator('.preview-img').waitFor()
  await page.waitForFunction(() => document.querySelector('.preview-img')?.naturalWidth > 0)
  assert.equal(await page.getByRole('button', { name: '快速下载', exact: true }).isEnabled(), true)
  await page.getByRole('button', { name: '预览视频封面', exact: true }).click()
  await page.locator('.t-image-viewer').waitFor()
  await page.keyboard.press('Escape')
  await page.evaluate(async () => {
    const { useUrlStore } = await import('/src/stores/urlStore.ts')
    useUrlStore().resetThumbnail()
  })
  await page.locator('.empty-preview').waitFor()
  assert.equal(await page.locator('.preview-img').count(), 0)
  await page.evaluate(async () => {
    const { useUrlStore } = await import('/src/stores/urlStore.ts')
    useUrlStore().setThumbnailUrl('data:image/png;base64,broken')
  })
  await page.getByText('封面暂时无法加载', { exact: true }).waitFor()
  await page.evaluate(async () => {
    const { useFormatStore } = await import('/src/stores/formatStore.ts')
    useFormatStore().isLoading = true
  })
  await page.getByText('正在分析视频…', { exact: true }).waitFor()
  assert.equal(await page.getByRole('button', { name: '分析中…', exact: true }).isDisabled(), true)
  await page.emulateMedia({ reducedMotion: 'reduce' })
  assert.equal(
    await page
      .locator('.is-spinning')
      .first()
      .evaluate((el) => getComputedStyle(el).animationName),
    'none',
  )
  await page.evaluate(async () => {
    const { useFormatStore } = await import('/src/stores/formatStore.ts')
    useFormatStore().isLoading = false
    const { useUrlStore } = await import('/src/stores/urlStore.ts')
    useUrlStore().resetThumbnail()
    const { useTaskStore } = await import('/src/stores/taskStore.ts')
    useTaskStore().updateTask({ taskId: 101, title: '测试任务', status: 'running', message: '运行中' })
  })
  assert.equal(await page.locator('.task-count').innerText(), '1')

  for (const viewport of [
    { width: 1280, height: 800 },
    { width: 960, height: 640 },
    { width: 640, height: 800 },
  ]) {
    await page.setViewportSize(viewport)
    for (const title of ['图片链接', '内容处理', '下载列表', '终端显示', '关于']) {
      await page.locator(`a.nav-item[title="${title}"]`).click()
      await page.getByRole('heading', { name: title, exact: true, level: 1 }).waitFor()
      await wait(100)
      const dimensions = await page.evaluate(() => {
        const sidebar = document.querySelector('.sidebar').getBoundingClientRect()
        const main = document.querySelector('.main-content').getBoundingClientRect()
        return {
          bodyOverflow: document.documentElement.scrollWidth > innerWidth,
          gap: main.left - sidebar.right,
          mainRight: main.right,
          width: innerWidth,
          contentOverflow:
            document.querySelector('.page-body').scrollWidth > document.querySelector('.page-body').clientWidth + 1,
        }
      })
      assert.equal(dimensions.contentOverflow, false, `Content overflow: ${title} at ${viewport.width}`)
      assert.equal(dimensions.bodyOverflow, false, `Horizontal overflow: ${title} at ${viewport.width}`)
      assert.ok(dimensions.gap >= 12 && dimensions.mainRight <= dimensions.width, JSON.stringify(dimensions))
      if (viewport.width === 1280)
        await page.screenshot({
          path: join(
            artifactDir,
            `page-${await page
              .locator('a.nav-item.is-active')
              .getAttribute('href')
              .then((href) => href.replaceAll(/[^a-z0-9]/g, '') || 'home')}-1280.png`,
          ),
          fullPage: true,
        })
    }
    await page.screenshot({ path: join(artifactDir, `about-${viewport.width}.png`), fullPage: true })
  }
  assert.deepEqual(errors, [], 'Unexpected runtime errors / Vue warnings')
  console.log(
    'UI smoke test passed: simplified navigation/about, persisted settings switch, tool profile dialog/browser guard, empty/loading/error previews, retained state, external links, reduced motion, task badge and all routes at 1280/960/640px.',
  )
  console.log('Screenshots: ' + artifactDir)
} finally {
  await browser?.close()
  server.kill()
}

