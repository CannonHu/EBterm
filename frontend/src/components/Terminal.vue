<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import { SearchAddon } from 'xterm-addon-search'
import { WebLinksAddon } from 'xterm-addon-web-links'
import { useResizeObserver, useDebounceFn } from '@vueuse/core'
import 'xterm/css/xterm.css'

interface Props {
  sessionId: string
  showTimestamp?: boolean
  theme?: 'dark' | 'light'
}

const props = withDefaults(defineProps<Props>(), {
  showTimestamp: false,
  theme: 'dark'
})

interface Emits {
  (e: 'data', data: string): void
  (e: 'resize', cols: number, rows: number): void
  (e: 'ready'): void
  (e: 'searchResults', result: { matchCount: number; currentMatch: number }): void
}

const emit = defineEmits<Emits>()

interface SearchResult {
  matchCount: number
  currentMatch: number
}

const searchResult = ref<SearchResult>({ matchCount: 0, currentMatch: 0 })

const containerRef = ref<HTMLElement | null>(null)

let terminal: Terminal | null = null
let fitAddon: FitAddon | null = null
let searchAddon: SearchAddon | null = null

const isReady = ref(false)
const currentCols = ref(80)
const currentRows = ref(24)

let writeBuffer: string[] = []
let writeTimeout: ReturnType<typeof setTimeout> | null = null
const WRITE_THROTTLE_MS = 16

const themes = {
  dark: {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
    cursor: '#d4d4d4',
    selectionBackground: '#264f78',
    black: '#000000',
    red: '#cd3131',
    green: '#0dbc79',
    yellow: '#e5e510',
    blue: '#2472c8',
    magenta: '#bc3fbc',
    cyan: '#11a8cd',
    white: '#e5e5e5'
  },
  light: {
    background: '#ffffff',
    foreground: '#333333',
    cursor: '#333333',
    selectionBackground: '#add6ff',
    black: '#000000',
    red: '#cd3131',
    green: '#00bc00',
    yellow: '#949800',
    blue: '#0451a5',
    magenta: '#bc05bc',
    cyan: '#0598bc',
    white: '#555555'
  }
}

function createTerminal(): void {
  if (!containerRef.value) return

  terminal = new Terminal({
    cols: currentCols.value,
    rows: currentRows.value,
    theme: themes[props.theme],
    cursorBlink: true,
    cursorStyle: 'block',
    fontFamily: 'Menlo, Monaco, "Courier New", monospace',
    fontSize: 14,
    lineHeight: 1.2,
    letterSpacing: 0,
    scrollback: 10000,
    fastScrollModifier: 'alt',
    fastScrollSensitivity: 5,
    windowsMode: false,
    unicodeVersion: '11',
    allowProposedApi: true
  })

  fitAddon = new FitAddon()
  searchAddon = new SearchAddon()

  // Set up search result event handling
  searchAddon.onDidChangeResults((result) => {
    searchResult.value = {
      matchCount: result.resultCount,
      currentMatch: result.resultIndex + 1 // Convert to 1-based indexing for display
    }
    emit('searchResults', searchResult.value)
  })

  terminal.loadAddon(fitAddon)
  terminal.loadAddon(searchAddon)
  terminal.loadAddon(new WebLinksAddon())

  terminal.open(containerRef.value)
  fitTerminal()
  setupInputHandling()

  isReady.value = true
  emit('ready')
}

function setupInputHandling(): void {
  if (!terminal) return

  terminal.onData((data) => {
    emit('data', data)
  })
}

function flushWriteBuffer(): void {
  if (!terminal || writeBuffer.length === 0) return

  const data = writeBuffer.join('')
  writeBuffer = []
  terminal.write(data)
}

function scheduleFlush(): void {
  if (writeTimeout) return

  writeTimeout = setTimeout(() => {
    flushWriteBuffer()
    writeTimeout = null
  }, WRITE_THROTTLE_MS)
}

function fitTerminal(): void {
  if (!fitAddon || !terminal) return

  try {
    fitAddon.fit()
    const dims = fitAddon.proposeDimensions()
    if (dims) {
      currentCols.value = dims.cols
      currentRows.value = dims.rows
      emit('resize', dims.cols, dims.rows)
    }
  } catch (err) {
    console.warn('Failed to fit terminal:', err)
  }
}

function write(data: string): void {
  if (!terminal) return

  let output = data

  if (props.showTimestamp) {
    const now = new Date()
    const hours = String(now.getHours()).padStart(2, '0')
    const minutes = String(now.getMinutes()).padStart(2, '0')
    const seconds = String(now.getSeconds()).padStart(2, '0')
    const ms = String(now.getMilliseconds()).padStart(3, '0')
    const timestamp = `[${hours}:${minutes}:${seconds}.${ms}] `
    output = timestamp + data
  }

  writeBuffer.push(output)
  scheduleFlush()
}

function writeln(data: string): void {
  write(data + '\r\n')
}

function clear(): void {
  if (!terminal) return
  terminal.clear()
}

function clearAll(): void {
  if (!terminal) return
  terminal.reset()
}

function focus(): void {
  if (!terminal) return
  terminal.focus()
}

function blur(): void {
  if (!terminal) return
  terminal.blur()
}

function getDimensions(): { cols: number; rows: number } {
  return {
    cols: currentCols.value,
    rows: currentRows.value
  }
}

function search(query: string): boolean {
  if (!searchAddon) return false
  return searchAddon.findNext(query, {
    decorations: {
      matchBackground: 'rgba(255, 255, 0, 0.3)',
      matchBorder: '#ffff00',
      activeMatchBackground: 'rgba(255, 136, 0, 0.5)',
      activeMatchBorder: '#ff8800',
    }
  })
}

function searchPrevious(query: string): boolean {
  if (!searchAddon) return false
  return searchAddon.findPrevious(query, {
    decorations: {
      matchBackground: 'rgba(255, 255, 0, 0.3)',
      matchBorder: '#ffff00',
      activeMatchBackground: 'rgba(255, 136, 0, 0.5)',
      activeMatchBorder: '#ff8800',
    }
  })
}

function getSearchMatches(): number {
  return searchResult.value.matchCount
}

function clearSearch(): void {
  if (!searchAddon) return
  searchAddon.clearDecorations()
}

const debouncedFit = useDebounceFn(fitTerminal, 100)
let resizeObserver: { stop: () => void } | null = null

onMounted(() => {
  createTerminal()

  if (containerRef.value) {
    resizeObserver = useResizeObserver(containerRef, () => {
      debouncedFit()
    })
  }
})

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.stop()
  }

  if (writeTimeout) {
    clearTimeout(writeTimeout)
    flushWriteBuffer()
  }

  if (terminal) {
    terminal.dispose()
    terminal = null
  }
  fitAddon = null
  searchAddon = null
})

watch(
  () => props.theme,
  (newTheme) => {
    if (terminal) {
      terminal.options.theme = themes[newTheme]
    }
  }
)

defineExpose({
  write,
  writeln,
  clear,
  clearAll,
  focus,
  blur,
  getDimensions,
  fit: fitTerminal,
  search,
  searchPrevious,
  getSearchMatches,
  clearSearch
})
</script>

<template>
  <div ref="containerRef" class="terminal-container" :class="{ 'is-ready': isReady }" />
</template>

<style scoped>
.terminal-container {
  width: 100%;
  height: 100%;
  min-height: 200px;
  background: v-bind("props.theme === 'dark' ? '#1e1e1e' : '#ffffff'");
  border-radius: 4px;
  overflow: hidden;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.terminal-container.is-ready {
  opacity: 1;
}

:deep(.xterm) {
  height: 100% !important;
  padding: 8px;
}

:deep(.xterm-screen) {
  height: 100% !important;
}
</style>
