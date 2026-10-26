import test from 'node:test'
import assert from 'node:assert/strict'
import { existsSync, readFileSync, readdirSync } from 'node:fs'

const repo = new URL('../../', import.meta.url)
const tauri = new URL('../src-tauri/', import.meta.url)
const read = (url) => readFileSync(url, 'utf8')

test('configured Windows bundle icons and project license exist', () => {
  const config = JSON.parse(read(new URL('tauri.conf.json', tauri)))
  const icons = new Set([...config.bundle.icon, config.bundle.windows.nsis.installerIcon, config.bundle.windows.nsis.uninstallerIcon])
  for (const icon of icons) assert.ok(existsSync(new URL(icon, tauri)), `Missing configured icon: ${icon}`)
  // Historical bin tools are optional development inputs, not installer resources.
  const license = Object.entries(config.bundle.resources).find(([, destination]) => destination === 'LICENSE')
  assert.ok(license, 'Project license must be bundled')
  assert.ok(existsSync(new URL(license[0], tauri)), 'Missing project license')
  // Raw artwork is intentionally ignored; a clean clone only needs bundle icons.
  assert.ok(existsSync(new URL('icons/icon.png', tauri)))
})

test('retired implementation and unused demo sources are not active repository files', () => {
  for (const path of ['legacy/python-eel/', 'frontend/src/stores/counter.ts', 'frontend/src/assets/test.jpg', 'frontend/app-icon-jpeg.png', 'test-write-permission.txt'])
    assert.equal(existsSync(new URL(path, repo)), false, `Retired file returned: ${path}`)
})

test('all relative Markdown document links point to existing files', () => {
  const docs = new URL('docs/', repo)
  const pages = [new URL('readme.md', repo), ...readdirSync(docs).filter((name) => name.endsWith('.md')).map((name) => new URL(name, docs))]
  for (const page of pages) {
    for (const [, href] of read(page).matchAll(/\[[^\]]*\]\(([^\s)]+)\)/g)) {
      if (/^[a-z][a-z\d+.-]*:/i.test(href) || href.startsWith('#') || href.startsWith('//')) continue
      const target = new URL(href, page)
      target.hash = ''
      target.search = ''
      assert.ok(existsSync(target), `${page.pathname}: broken link ${href}`)
    }
  }
})

test('current and future release notes use user-facing sections without validation reports', () => {
  const docs = new URL('docs/', repo)
  // Release notes are maintained on GitHub; this repository keeps only the template.
  const notes = readdirSync(docs).filter((name) => {
    if (name === 'RELEASE_TEMPLATE.md') return true
    const match = /^RELEASE_v(\d+)\.(\d+)\.(\d+)\.md$/.exec(name)
    if (!match) return false
    const [major, minor, patch] = match.slice(1).map(Number)
    return major > 2 || (major === 2 && (minor > 0 || patch >= 1))
  })
  assert.ok(notes.includes('RELEASE_TEMPLATE.md'), 'Release template is required')
  for (const name of notes) {
    const text = read(new URL(name, docs))
    for (const section of ['更新内容', '下载与安装', '使用须知', '文件信息'])
      assert.ok(text.includes('## ' + section), `${name}: missing ${section}`)
    assert.doesNotMatch(text, /^#{1,6}\s+.*(?:验证|测试|验收|\b(?:validation|verification|tests?|testing)\b)/im, name)
    assert.doesNotMatch(text, /自动化测试|单元测试|烟测|\b(?:clippy|ESLint)\b|cargo (?:fmt|test)|\d+\s*\/\s*\d+\s*通过/i, name)
  }
})

test('installer test overlay isolates product, executable, app data and skips WebView2 changes', () => {
  const production = JSON.parse(read(new URL('tauri.conf.json', tauri)))
  const isolated = JSON.parse(read(new URL('tauri.install-test.conf.json', tauri)))
  assert.notEqual(isolated.productName, production.productName)
  assert.notEqual(isolated.identifier, production.identifier)
  assert.notEqual(isolated.mainBinaryName, 'grabmeta')
  assert.equal(isolated.productName, 'GrabMeta Install Test')
  assert.equal(isolated.bundle.windows.nsis.startMenuFolder, isolated.productName)
  assert.equal(isolated.bundle.windows.webviewInstallMode.type, 'skip')
  assert.equal(isolated.bundle.windows.webviewInstallMode.silent, null, 'JSON merge patch must remove the production silent field for skip')
  assert.equal(production.bundle.windows.webviewInstallMode.type, 'downloadBootstrapper')
  assert.ok(!isolated.bundle.resources, 'Test build must inherit production resources')
})

test('resource smoke test guards the isolated identity and hides installer helpers', () => {
  const script = read(new URL('../scripts/smoke-install-resources.ps1', import.meta.url))
  assert.match(script, /com\.ssk-shandm\.grabmeta\.install-test/)
  assert.match(script, /refusing to overwrite/)
  assert.match(script, /Unsafe uninstall target/)
  assert.match(script, /-WindowStyle Hidden/)
  assert.match(script, /productionAndWebView2Unchanged/)
  assert.doesNotMatch(script, /Remove-Item|cmd \/c/i)
})


test('real tool smoke defaults to transfer-only; full installs require an explicit flag', () => {
  const script = read(new URL('../scripts/smoke-tools.mjs', import.meta.url))
  assert.ok(script.includes("process.argv.includes('--full')"))
  assert.ok(script.includes('if (!full)'))
  assert.match(script, /transfer-only/)
  assert.match(script, /proxyEnabled: true, proxyUrl/)
})
