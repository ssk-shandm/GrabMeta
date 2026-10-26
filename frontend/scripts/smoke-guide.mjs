import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdtemp } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const port = 1422
const baseUrl = `http://127.0.0.1:${port}`
const artifactDir = await mkdtemp(join(tmpdir(), 'grabmeta-guide-'))
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
  const guideSteps = [
    ['粘贴视频链接', '#video-url', '/'],
    ['分析链接', '.analyze-btn', '/'],
    ['快速下载', '.quick-btn', '/'],
    ['自定义下载', 'a[href="#/page-two"]', '/page-two'],
    ['设置下载目录', '[data-guide="download-directory"]', '/page-two'],
    ['设置重试次数', '[data-guide="retry-limit"]', '/page-two'],
    ['下载字幕', '[data-guide="subtitles"] .box-heading', '/page-two'],
    ['选择视频和音频', '[data-guide="video-audio-quality"]', '/page-two'],
    ['选择输出格式', '[data-guide="container-format"]', '/page-two'],
    ['开始自定义下载', '[data-guide="custom-download"]', '/page-two'],
    ['筛选可用格式', '[data-guide="format-filter"]', '/page-two'],
    ['查看任务', 'a[href="#/page-three"]', '/page-three'],
    ['查看详细日志', 'a[href="#/page-four"]', '/page-four'],
    ['复制日志排查问题', '[data-guide="copy-logs"]', '/page-four'],
    ['安装下载工具', '[data-guide="install-tools"]', '/about'],
    ['检查软件更新', '[data-guide="check-updates"]', '/about'],
  ]
  async function assertSpotlight(selector) {
    await page.waitForFunction((selector) => {
      const element = document.querySelector(selector)
      const highlight = document.querySelector('.guide-highlight')
      if (!element || !highlight) return false
      const target = element.getBoundingClientRect()
      const ring = highlight.getBoundingClientRect()
      return (
        target.width > 0 &&
        target.height > 0 &&
        Math.abs(ring.left - (target.left - 6)) < 1 &&
        Math.abs(ring.top - (target.top - 6)) < 1 &&
        Math.abs(ring.width - (target.width + 12)) < 1 &&
        Math.abs(ring.height - (target.height + 12)) < 1
      )
    }, selector)
    const geometry = await page.evaluate((selector) => {
      const target = document.querySelector(selector).getBoundingClientRect()
      const card = document.querySelector('.guide-card').getBoundingClientRect()
      return {
        dimmer: getComputedStyle(document.querySelector('.guide-dimmer')).backgroundColor,
        overlapping:
          target.left < card.right && target.right > card.left && target.top < card.bottom && target.bottom > card.top,
        cardInViewport: card.left >= 0 && card.top >= 0 && card.right <= innerWidth && card.bottom <= innerHeight,
        targetInViewport:
          target.left >= 0 && target.top >= 0 && target.right <= innerWidth && target.bottom <= innerHeight,
      }
    }, selector)
    assert.equal(geometry.dimmer, 'rgba(0, 0, 0, 0)', 'The dimmer must not cover the spotlight center')
    assert.equal(geometry.overlapping, false, 'The tutorial card must not cover its target')
    assert.ok(geometry.cardInViewport && geometry.targetInViewport, JSON.stringify(geometry))
  }
  for (const viewport of [
    { width: 1280, height: 800 },
    { width: 960, height: 640 },
    { width: 640, height: 480 },
  ]) {
    await page.setViewportSize(viewport)
    await page.locator('a.nav-item[title="关于"]').click()
    await page.locator('.about-page').waitFor()
    await page.getByRole('button', { name: '新手教程', exact: true }).click()
    for (const [index, [title, selector, path]] of guideSteps.entries()) {
      await page.locator('#guide-title').filter({ hasText: title }).waitFor()
      await page.waitForFunction(() => !document.querySelector('.next-action').disabled)
      await assertSpotlight(selector)
      assert.equal(
        await page.locator('.guide-card .section-eyebrow').innerText(),
        `新手教程 · ${index + 1} / ${guideSteps.length}`,
      )
      assert.equal(await page.evaluate(() => location.hash), `#${path}`)
      // Compare rendered pixels to catch regressions where the ring exists but its target is dimmed.
      if (viewport.width === 1280) {
        await page.mouse.move(0, 0)
        await wait(300)
        const clip = await page.locator(selector).boundingBox()
        const highlighted = await page.screenshot({ clip })
        await page.locator('.guide-layer').evaluate((element) => {
          element.style.visibility = 'hidden'
        })
        const unobscured = await page.screenshot({ clip })
        await page.locator('.guide-layer').evaluate((element) => {
          element.style.visibility = ''
        })
        // Compare decoded pixels, not PNG compression bytes. A compositor
        // rounding difference of one channel level is not an obscured target.
        const maxPixelDelta = await page.evaluate(async (images) => {
          const pixels = await Promise.all(images.map(async (base64) => {
            const image = new Image()
            image.src = 'data:image/png;base64,' + base64
            await image.decode()
            const canvas = document.createElement('canvas')
            canvas.width = image.width; canvas.height = image.height
            const context = canvas.getContext('2d')
            context.drawImage(image, 0, 0)
            return context.getImageData(0, 0, canvas.width, canvas.height).data
          }))
          let delta = 0
          for (let i = 0; i < pixels[0].length; i++) delta = Math.max(delta, Math.abs(pixels[0][i] - pixels[1][i]))
          return delta
        }, [highlighted.toString('base64'), unobscured.toString('base64')])
        if (maxPixelDelta > 1) {
          const { writeFile } = await import('node:fs/promises')
          await writeFile(join(artifactDir, 'highlighted.png'), highlighted)
          await writeFile(join(artifactDir, 'unobscured.png'), unobscured)
          await page.screenshot({ path: join(artifactDir, 'debug.png') })
          console.log(artifactDir)
        }
        assert.ok(maxPixelDelta <= 1, `Spotlight target is still dimmed: ${title}`)
        if (index === 1 || index === 2)
          await page.screenshot({ path: join(artifactDir, `guide-step-${index + 1}-${viewport.width}.png`) })
      }
      if (index === 0) {
        await page.locator('.guide-card').focus()
        await page.keyboard.press('Tab')
        assert.equal(await page.locator('.skip-action').evaluate((element) => element === document.activeElement), true)
        await page.keyboard.press('Shift+Tab')
        assert.equal(await page.locator('.next-action').evaluate((element) => element === document.activeElement), true)
        if (viewport.width === 640) {
          await page.evaluate(() => {
            document.querySelector('.page-body').scrollTop += 16
          })
          await assertSpotlight(selector)
          await page.setViewportSize({ width: 700, height: 520 })
          await assertSpotlight(selector)
          await page.setViewportSize(viewport)
          await assertSpotlight(selector)
        }
      }
      if (index === 2) {
        assert.equal(await page.locator('.quick-btn').isDisabled(), true)
        await page.getByRole('button', { name: '上一步', exact: true }).click()
        await page.locator('#guide-title').filter({ hasText: '分析链接' }).waitFor()
        await assertSpotlight('.analyze-btn')
        await page.getByRole('button', { name: '下一步', exact: true }).click()
        await assertSpotlight(selector)
      }
      await page.locator('.next-action').click()
    }
    await page.locator('.guide-layer').waitFor({ state: 'detached' })
    assert.equal(await page.locator('.guide-highlight').count(), 0)
    assert.equal(await page.locator('.guide-button').evaluate((element) => element === document.activeElement), true)
    await page.getByRole('button', { name: '新手教程', exact: true }).click()
    await assertSpotlight('#video-url')
    await page.keyboard.press('Escape')
    await page.locator('.guide-layer').waitFor({ state: 'detached' })
    await page.getByRole('button', { name: '新手教程', exact: true }).click()
    await assertSpotlight('#video-url')
    await page.getByRole('button', { name: '跳过', exact: true }).click()
    await page.locator('.guide-layer').waitFor({ state: 'detached' })
  }
  // Rapid input must not advance twice or leave a spotlight behind after dismissal.
  await page.getByRole('button', { name: '新手教程', exact: true }).click()
  await assertSpotlight('#video-url')
  await page.evaluate(() => {
    const next = document.querySelector('.next-action')
    next.click()
    next.click()
  })
  await page.locator('#guide-title').filter({ hasText: '分析链接' }).waitFor()
  await assertSpotlight('.analyze-btn')
  await page.locator('.guide-dimmer').click({ position: { x: 4, y: 4 } })
  await page.locator('.guide-layer').waitFor({ state: 'detached' })
  await wait(150)
  assert.equal(await page.locator('.guide-highlight').count(), 0)
  assert.deepEqual(errors, [], 'Unexpected runtime errors / Vue warnings')
  console.log(
    'Tutorial smoke test passed: all sixteen spotlights, unobscured target pixels, disabled buttons, back/next, completion, Escape/skip, focus, scrolling and resizing at 1280/960/640px.',
  )
  console.log('Screenshots: ' + artifactDir)
} finally {
  await browser?.close()
  server.kill()
}
