import { ref } from 'vue'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import { WebLinksAddon } from 'xterm-addon-web-links'
import { SearchAddon } from 'xterm-addon-search'
import 'xterm/css/xterm.css'

export function useTerminal() {
  const terminal = ref<Terminal | null>(null)
  const fitAddon = ref<FitAddon | null>(null)

  const init = (container: HTMLElement) => {
    terminal.value = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, monospace',
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4'
      }
    })

    fitAddon.value = new FitAddon()
    terminal.value.loadAddon(fitAddon.value)
    terminal.value.loadAddon(new WebLinksAddon())
    terminal.value.loadAddon(new SearchAddon())

    terminal.value.open(container)
  }

  const fit = () => {
    fitAddon.value?.fit()
  }

  const write = (data: string) => {
    terminal.value?.write(data)
  }

  const dispose = () => {
    terminal.value?.dispose()
    terminal.value = null
    fitAddon.value = null
  }

  return {
    terminal,
    fitAddon,
    init,
    fit,
    write,
    dispose
  }
}
