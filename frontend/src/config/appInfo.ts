import { version } from '../../package.json'
export const APP_VERSION = version
export const COPYRIGHT_LINE = 'Copyright © 2025 ssk-shandm'
export const LICENSE_NAME = 'MIT License'
export const REPOSITORY_URL = 'https://github.com/ssk-shandm/GrabMeta'
export const RELEASES_URL = `${REPOSITORY_URL}/releases`
export const LICENSE_URL = `${REPOSITORY_URL}/blob/main/LICENSE`
export const YTDLP_URL = 'https://github.com/yt-dlp/yt-dlp'
export const FFMPEG_URL = 'https://ffmpeg.org/'
export const FFMPEG_BUILDS_URL = 'https://github.com/BtbN/FFmpeg-Builds'
export const ABOUT_URLS = [REPOSITORY_URL, RELEASES_URL, LICENSE_URL, YTDLP_URL, FFMPEG_URL, FFMPEG_BUILDS_URL]
export const APP_PAGES = [
  {
    path: '/',
    titleKey: 'common.pages.links.title',
    eyebrowKey: 'common.pages.links.eyebrow',
    descriptionKey: 'common.pages.links.description',
    icon: 'image',
  },
  {
    path: '/page-two',
    titleKey: 'common.pages.content.title',
    eyebrowKey: 'common.pages.content.eyebrow',
    descriptionKey: 'common.pages.content.description',
    icon: 'video',
  },
  {
    path: '/page-three',
    titleKey: 'common.pages.downloads.title',
    eyebrowKey: 'common.pages.downloads.eyebrow',
    descriptionKey: 'common.pages.downloads.description',
    icon: 'download',
  },
  {
    path: '/page-four',
    titleKey: 'common.pages.terminal.title',
    eyebrowKey: 'common.pages.terminal.eyebrow',
    descriptionKey: 'common.pages.terminal.description',
    icon: 'terminal',
  },
  {
    path: '/about',
    titleKey: 'common.pages.about.title',
    eyebrowKey: 'common.pages.about.eyebrow',
    descriptionKey: 'common.pages.about.description',
    icon: 'info',
  },
] as const
