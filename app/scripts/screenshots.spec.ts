import { test, expect } from '@playwright/test';
import path from 'node:path';
const workspaces = [
  'Overview',
  'Explorer',
  'Texture & 2D',
  'Text',
  'Track',
  'Model',
  'Rig & Animate',
  'Audio',
  'Code',
  'Test Bench',
];
for (const theme of ['dark', 'light'])
  for (const workspace of workspaces) {
    test(`${workspace} ${theme}`, async ({ page }) => {
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(String(e)));
      await page.goto(`/?workspace=${encodeURIComponent(workspace)}`);
      await expect(page.getByTestId('workspace')).toBeVisible();
      await expect(page.getByText('SYNTHETIC DEMO')).toBeVisible();
      await page.getByRole('button', { name: 'Settings', exact: true }).click();
      await page.getByLabel('Theme', { exact: true }).selectOption(theme);
      await page.getByRole('button', { name: 'Save settings', exact: true }).click();
      await expect(page.getByRole('dialog', { name: 'Settings', exact: true })).toBeHidden();
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      if (workspace === 'Text') {
        await page.getByLabel('BMG file').selectOption('data/Studio/demo.carc/language/en.bmg');
        await expect(
          page.getByText('Welcome to your next idea.', { exact: true }).first(),
        ).toBeVisible();
      }
      if (workspace === 'Texture & 2D') {
        await expect(page.getByRole('button', { name: 'Colour 0', exact: true })).toBeVisible();
        await expect(page.getByLabel('Texture preview')).toHaveAttribute('width', '128');
      }
      if (workspace === 'Model' || workspace === 'Track') {
        await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
        if (workspace === 'Track') await expect(page.getByLabel('Show Collision')).toBeChecked();
      }
      if (workspace === 'Test Bench') {
        await page.getByRole('button', { name: 'Refresh courses' }).click();
        await page.getByLabel('Race course').selectOption('0');
      }
      await page.evaluate(() => document.fonts.ready);
      await page.screenshot({
        path: path.resolve(
          `../docs/screenshots/${workspace.toLowerCase().replaceAll(/[^a-z0-9]+/g, '-')}-${theme}.png`,
        ),
        fullPage: true,
        animations: 'disabled',
      });
      expect(errors).toEqual([]);
    });
  }
