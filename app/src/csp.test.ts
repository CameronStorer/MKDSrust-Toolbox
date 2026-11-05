import { describe, expect, it } from 'vitest';
import config from '../src-tauri/tauri.conf.json';
describe('content security policy', () => {
  it('lets three.js fetch embedded glTF textures (GLTFLoader fetches blob: URLs)', () => {
    const csp: string = config.app.security.csp;
    const connect = csp.split(';').find((d) => d.trim().startsWith('connect-src')) ?? '';
    expect(connect.split(/\s+/)).toContain('blob:');
  });
});
