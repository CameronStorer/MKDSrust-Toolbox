import { test, expect } from '@playwright/test';
import path from 'node:path';
test('Model animation, Track selection and Test Bench restart', async ({ page }) => {
  await page.goto('/?workspace=Model');
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await page.getByLabel('Animation', { exact: true }).selectOption('0');
  await page.getByRole('button', { name: 'Play animation', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Pause animation' })).toBeVisible();
  await page.getByRole('button', { name: 'Track', exact: true }).click();
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await page.getByRole('button', { name: 'OBJI 0 · id 42', exact: true }).click();
  await expect(page.getByText('Object id', { exact: true })).toBeVisible();
  await page.getByLabel('Show Collision').uncheck();
  await expect(page.getByLabel('Show Collision')).not.toBeChecked();
  await page.getByRole('button', { name: 'Test Bench', exact: true }).click();
  await page.getByRole('button', { name: 'Refresh courses' }).click();
  await page.getByLabel('Race course').selectOption('0');
  await page.getByRole('button', { name: 'Play target', exact: true }).click();
  await expect(page.getByText('● Running')).toBeVisible();
  await page.keyboard.press('Control+r');
  await expect(page.getByText('● Running')).toBeVisible();
  await page.getByRole('button', { name: 'Stop game', exact: true }).click();
  await expect(page.getByText('○ Stopped')).toBeVisible();
});
test('paint stroke is undoable; PNG conversion previews before apply', async ({ page }) => {
  await page.goto('/?workspace=Texture%20%26%202D');
  const canvas = page.getByLabel('Texture preview');
  await expect(canvas).toHaveAttribute('width', '128');
  await page.getByRole('button', { name: 'Tool pencil', exact: true }).click();
  await page.getByLabel('Paint palette index').fill('2');
  const pixel = () =>
    canvas.evaluate((c) =>
      Array.from((c as HTMLCanvasElement).getContext('2d')!.getImageData(0, 0, 1, 1).data),
    );
  const before = await pixel();
  const box = (await canvas.boundingBox())!;
  await page.mouse.click(box.x + 1, box.y + 1);
  await expect.poll(pixel).not.toEqual(before);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect.poll(pixel).toEqual(before);
  await page.getByRole('button', { name: 'Import PNG…', exact: true }).click();
  await page.getByRole('button', { name: 'Preview conversion', exact: true }).click();
  await expect(page.getByAltText('After conversion')).toBeVisible();
  await page.screenshot({ path: path.resolve('../docs/screenshots/painting-import-dark.png') });
  await page.getByRole('button', { name: 'Apply PNG import', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Apply PNG import', exact: true })).toBeHidden();
});
test('DS export, imported base provenance and cell playback', async ({ page }) => {
  await page.goto('/?workspace=Overview');
  await page.getByRole('button', { name: 'Export for DS…', exact: true }).click();
  await page.getByRole('button', { name: 'Choose ROM output…', exact: true }).click();
  await page.getByRole('button', { name: 'Choose patch output…', exact: true }).click();
  await page.getByRole('button', { name: 'Build DS output', exact: true }).click();
  await expect(page.getByText('Export complete', { exact: true })).toBeVisible();
  await page.screenshot({ path: path.resolve('../docs/screenshots/export-ds-dark.png') });
  await page
    .getByRole('dialog', { name: 'Export for DS', exact: true })
    .getByRole('button', { name: 'Close', exact: true })
    .click();
  await page.keyboard.press('F3');
  await page.getByLabel('Search commands').fill('Create project');
  await page.keyboard.press('Enter');
  await page.getByLabel('Project base').selectOption('bps');
  await page.getByRole('button', { name: 'Choose base…', exact: true }).click();
  await page.getByRole('button', { name: 'Choose folder…', exact: true }).click();
  await page
    .getByRole('dialog', { name: 'New project', exact: true })
    .getByRole('button', { name: 'Create project', exact: true })
    .click();
  await page.getByRole('button', { name: 'Overview', exact: true }).click();
  await expect(page.getByText('What this hack changes', { exact: true })).toBeVisible();
  await page.screenshot({ path: path.resolve('../docs/screenshots/imported-base-dark.png') });
  await page.getByRole('button', { name: 'Explorer', exact: true }).click();
  await page.getByPlaceholder('Filter files…').fill('studio.nanr');
  await page.getByRole('treeitem', { name: /studio.nanr/ }).dblclick();
  await expect(page.getByAltText('Cell composition')).toBeVisible();
  await page.getByRole('button', { name: 'Play cells', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Pause cells', exact: true })).toBeVisible();
});
