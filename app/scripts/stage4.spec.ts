import { test, expect } from '@playwright/test';

test('model tools use project history and collision has generation and brush controls', async ({
  page,
}) => {
  await page.goto('/?workspace=Model');
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await page.getByRole('button', { name: 'Translate', exact: true }).click();
  const modelViewport = await page.getByTestId('viewport').boundingBox();
  if (!modelViewport) throw new Error('model viewport has no bounds');
  await page.mouse.move(
    modelViewport.x + modelViewport.width * 0.5,
    modelViewport.y + modelViewport.height * 0.52,
  );
  await page.mouse.down();
  await page.mouse.move(
    modelViewport.x + modelViewport.width * 0.5,
    modelViewport.y + modelViewport.height * 0.46,
    { steps: 8 },
  );
  await page.mouse.up();
  await expect(page.getByText('Transform model', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Import model…' })).toBeEnabled();
  await page.getByRole('button', { name: 'Import model…' }).click();
  await expect(page.getByText('Import model', { exact: true })).toBeVisible();
  await page.getByLabel('Target triangles').fill('1');
  await page.getByRole('button', { name: 'Decimate', exact: true }).click();
  await expect(page.getByText('Decimate model', { exact: true })).toBeVisible();
  await expect(page.getByLabel('Model part')).toContainText('1 triangles');
  await page.keyboard.press('Control+z');
  await expect(page.getByLabel('Model part')).toContainText('4 triangles');
  await page.getByRole('button', { name: 'Cut by plane' }).click();
  await expect(page.getByText('Cut model', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Track', exact: true }).click();
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await page.getByRole('button', { name: 'Generate collision', exact: true }).click();
  await expect(page.getByText('Generate collision', { exact: true }).last()).toBeVisible();
  await page.getByLabel('Collision type', { exact: true }).selectOption('3');
  await expect(page.getByLabel('Collision attribute')).toHaveValue(String(0x8300));
  await page.getByRole('button', { name: 'Paint collision', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Brush active' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  const viewport = await page.getByTestId('viewport').boundingBox();
  if (!viewport) throw new Error('viewport has no bounds');
  await page.mouse.move(viewport.x + viewport.width * 0.5, viewport.y + viewport.height * 0.7);
  await page.mouse.down();
  await page.mouse.move(viewport.x + viewport.width * 0.55, viewport.y + viewport.height * 0.72, {
    steps: 5,
  });
  await page.mouse.up();
  await expect(page.getByText('Paint collision', { exact: true })).toBeVisible();
});

test('audio tree, sample preview and sampled import report', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto('/?workspace=Audio');
  await expect(page.getByRole('tree', { name: 'SDAT tree' })).toBeVisible();
  await expect(page.getByRole('treeitem', { name: /Original synth tone/ })).toBeVisible();
  await page.getByRole('treeitem', { name: /Sine waves/ }).click();
  await page.getByRole('button', { name: 'Play preview' }).click();
  await expect(page.getByLabel('Audio preview')).toHaveAttribute('src', /^blob:/);
  await page.getByRole('button', { name: 'Choose WAV / MIDI…' }).click();
  await page.getByLabel('Loop start', { exact: true }).fill('0');
  await page.getByLabel('Song route').selectOption('sampled');
  await page.getByRole('button', { name: 'Import song', exact: true }).click();
  await expect(page.getByText(/Imported sampled · 8 KiB RAM/)).toBeVisible();
  expect(errors).toEqual([]);
});
