import {
  commands,
  type Request,
  type Snapshot,
  type BinaryRequest,
  type Preview,
} from './bindings';
import type { SceneInfo } from './bindings';
export async function inspectScene(path: string): Promise<SceneInfo> {
  if (isMock) return (await import('./mock')).scene(path);
  const result = await commands.scene(path);
  if (result.status === 'error') throw new Error(result.error);
  return result.data;
}
export const isMock = import.meta.env.VITE_MOCK_BACKEND === '1';
export async function inspect(path: string): Promise<Preview> {
  if (isMock) return (await import('./mock')).inspect(path);
  const result = await commands.inspect(path);
  if (result.status === 'error') throw new Error(result.error);
  return result.data;
}
export async function dispatch(request: Request): Promise<Snapshot> {
  if (isMock) return (await import('./mock')).dispatch(request);
  const result = await commands.dispatch(request);
  if (result.status === 'error') throw new Error(result.error);
  return result.data;
}
export async function binary(request: BinaryRequest): Promise<ArrayBuffer> {
  if (isMock) return (await import('./mock')).binary(request);
  const result = await commands.binary(request);
  if (result.status === 'error') throw new Error(result.error);
  const response = await fetch(result.data);
  if (!response.ok) throw new Error('The binary preview expired. Select the file again.');
  return response.arrayBuffer();
}
export async function pickDirectory(): Promise<string | null> {
  if (isMock) return '/projects/colour-study';
  return (await import('@tauri-apps/plugin-dialog')).open({ directory: true });
}
export async function pickRom(): Promise<string | null> {
  if (isMock) return '/user/roms/synthetic.nds';
  return (await import('@tauri-apps/plugin-dialog')).open({
    filters: [{ name: 'Nintendo DS image', extensions: ['nds'] }],
  });
}
export async function savePath(defaultPath: string): Promise<string | null> {
  if (isMock) return `/exports/${defaultPath}`;
  return (await import('@tauri-apps/plugin-dialog')).save({ defaultPath });
}
