import { test, expect } from '@playwright/test';
import path from 'node:path';

for (const theme of ['dark', 'light']) {
  for (const flow of ['Character', 'Kart', 'Track', 'Music', 'Cups', 'Items', 'Names', 'Tables']) {
    test(`recipe ${flow} ${theme}`, async ({ page }) => {
      const errors: string[] = [];
      page.on('pageerror', (e) => errors.push(String(e)));
      await page.goto('/?workspace=Overview');
      await page.getByRole('button', { name: 'Settings', exact: true }).click();
      await page.getByLabel('Theme', { exact: true }).selectOption(theme);
      await page.getByRole('button', { name: 'Save settings', exact: true }).click();
      await page.getByRole('button', { name: 'Beginner recipes', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: 'Beginner recipes' });
      await dialog
        .getByRole('navigation', { name: 'Recipe types' })
        .getByRole('button', { name: flow, exact: true })
        .click();
      if (['Character', 'Kart', 'Track', 'Music', 'Items'].includes(flow)) {
        await page
          .getByLabel('Recipe source')
          .fill(`/author/studio.${flow === 'Music' ? 'wav' : flow === 'Items' ? 'png' : 'obj'}`);
      }
      if (flow === 'Names') {
        await page.getByLabel('Search names').fill('Studio name 37');
        await dialog.getByRole('button', { name: /Studio name 37/ }).click();
        await page.getByLabel('New name').fill('Studio Cup');
      }
      if (flow === 'Track') {
        await expect(page.getByLabel('Draw racing line')).toBeVisible();
        await dialog.getByRole('button', { name: 'Remove last point' }).click();
        await dialog.getByRole('button', { name: 'Default loop' }).click();
      }
      await dialog
        .getByRole('button', { name: `Apply ${flow.toLowerCase()}`, exact: true })
        .click();
      await expect(dialog.getByText(/Applied · \d+ files/)).toBeVisible();
      await dialog.locator('.wizard-body').evaluate((element) => {
        element.scrollTop = 0;
      });
      await page.evaluate(() => document.fonts.ready);
      await page.screenshot({
        path: path.resolve(`../docs/screenshots/recipe-${flow.toLowerCase()}-${theme}.png`),
        fullPage: true,
        animations: 'disabled',
      });
      await page.getByLabel('Close recipes').click();
      await page.keyboard.press('Control+z');
      await page.keyboard.press('Control+Shift+z');
      expect(errors).toEqual([]);
    });
  }
}
