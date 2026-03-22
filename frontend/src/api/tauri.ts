import { invoke } from '@tauri-apps/api/tauri';
import type { CommandResult } from '../types/ipc';

export async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<CommandResult<T>> {
  const result = await invoke<CommandResult<T>>(cmd, args);
  return result;
}
