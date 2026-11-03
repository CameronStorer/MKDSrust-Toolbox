import { build, createServer, preview, type InlineConfig } from 'vite';

// Own Vite directly so teardown does not depend on terminating a shell process tree.
export default async function setup() {
  if (process.env.MKDSTB_REUSE_SERVER === '1') return;
  process.env.VITE_MOCK_BACKEND = '1';
  const options: InlineConfig = { configLoader: 'runner' };
  const address = { host: '127.0.0.1', port: 1420, strictPort: true };
  if (process.env.MKDSTB_SCREENSHOT_PREVIEW === '1') {
    const previewOptions: InlineConfig = { ...options, build: { outDir: '.cache/screenshots-dist' } };
    await build(previewOptions);
    const server = await preview({ ...previewOptions, preview: address });
    return async () => {
      if ('closeAllConnections' in server.httpServer) server.httpServer.closeAllConnections();
      await new Promise<void>((resolve, reject) =>
        server.httpServer.close((error) => (error ? reject(error) : resolve())),
      );
    };
  }
  const server = await createServer({ ...options, server: address });
  await server.listen();
  return async () => {
    if (server.httpServer && 'closeAllConnections' in server.httpServer)
      server.httpServer.closeAllConnections();
    await server.close();
  };
}
