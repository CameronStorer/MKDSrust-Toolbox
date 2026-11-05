import { test, expect } from '@playwright/test';
import path from 'node:path';

test('Explorer expands, collapses, groups and keeps its view across workspaces', async ({
  page,
}) => {
  await page.goto('/?workspace=Explorer');
  const tree = page.getByRole('tree', { name: 'ROM files' });
  await page.getByRole('button', { name: 'Collapse all', exact: true }).click();
  await expect(tree.getByRole('treeitem')).toHaveCount(1);
  await page.getByRole('button', { name: 'Expand all', exact: true }).click();
  await page.getByLabel('Search files').fill('Main2D_es');
  await expect(tree.getByRole('treeitem')).toHaveCount(0);
  await page.getByLabel('Show other languages').check();
  await expect(tree.getByRole('treeitem').filter({ hasText: 'common.bmg' })).toBeVisible();
  await page.getByLabel('Group by type').uncheck();
  await page.getByRole('button', { name: 'Overview', exact: true }).click();
  await page.getByRole('button', { name: 'Explorer', exact: true }).click();
  await expect(page.getByLabel('Search files')).toHaveValue('Main2D_es');
  await expect(page.getByLabel('Show other languages')).toBeChecked();
  await expect(page.getByLabel('Group by type')).not.toBeChecked();
  await tree.getByRole('treeitem').filter({ hasText: 'common.bmg' }).dblclick();
  await expect(page.getByLabel('BMG file')).toHaveValue('data/Main2D_es.carc/common.bmg');
});

test('related character, kart and track pages preview independently and replace the chosen slot', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto('/?workspace=Related%20assets');
  await expect(page.getByRole('heading', { name: 'Mario', exact: true })).toBeVisible();
  const driver = page.getByRole('article', { name: 'Race driver', exact: true });
  await expect(driver.getByLabel('Race driver viewport')).toHaveAttribute('data-loaded', 'true');
  await driver.getByRole('button', { name: 'Pause', exact: true }).click();
  await expect(driver.getByRole('button', { name: 'Play', exact: true })).toBeVisible();
  const voice = page.getByRole('article', { name: 'Voice', exact: true });
  await voice.scrollIntoViewIfNeeded();
  await voice.getByRole('button', { name: 'Play preview' }).click();
  await expect(voice.locator('audio')).toHaveAttribute('src', /^blob:/);
  await page.getByRole('button', { name: 'Mario kart 2', exact: true }).first().click();
  const body = page.getByRole('article', { name: 'Race body', exact: true });
  await expect(body.getByLabel('Race body viewport')).toHaveAttribute('data-loaded', 'true');
  await page.getByRole('button', { name: 'Replace kart…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Beginner recipes' });
  await expect(dialog.getByLabel('Replace kart id', { exact: true })).toHaveValue('1');
  await page.getByLabel('Close recipes').click();
  await page.getByLabel('Search related assets').fill('Studio name 45');
  await page
    .locator('.related-nav')
    .getByRole('button', { name: 'Studio name 45', exact: true })
    .click();
  await expect(
    page
      .getByRole('article', { name: 'Collision overlay', exact: true })
      .getByLabel('Collision overlay viewport'),
  ).toHaveAttribute('data-loaded', 'true');
  await page
    .getByRole('article', { name: 'Routes and objects', exact: true })
    .scrollIntoViewIfNeeded();
  await expect(page.getByLabel('Routes and objects viewport')).toHaveAttribute(
    'data-loaded',
    'true',
  );
  await page.screenshot({ path: path.resolve('../docs/screenshots/stage6b-related-track.png') });
  expect(errors).toEqual([]);
});

test('Explorer opens unknown files in a popup with Export', async ({ page }) => {
  await page.goto('/?workspace=Explorer');
  await page.getByLabel('Search files').fill('kartoffsetdata.bin');
  await page.getByRole('treeitem').filter({ hasText: 'kartoffsetdata.bin' }).first().dblclick();
  const dialog = page.getByRole('dialog', { name: 'File preview' });
  await expect(dialog.getByRole('button', { name: 'Export file…' })).toBeVisible();
  await dialog.getByRole('button', { name: 'Close preview' }).click();
  await expect(dialog).not.toBeVisible();
  await page.screenshot({ path: path.resolve('../docs/screenshots/stage6b-explorer.png') });
});

test('image and model links load their automatic pairs with manual image overrides', async ({
  page,
}) => {
  await page.goto('/?workspace=Explorer');
  await page.getByLabel('Search files').fill('art/studio.ncgr');
  await page.getByRole('treeitem').filter({ hasText: 'studio.ncgr' }).dblclick();
  await expect(page.getByLabel('Palette source')).toHaveValue(
    'data/Studio/demo.carc/art/studio.nclr',
  );
  await expect(page.getByLabel('Screen source')).toHaveValue(
    'data/Studio/demo.carc/art/studio.nscr',
  );
  await page.getByLabel('Screen source').selectOption('');
  await expect(page.getByLabel('Screen source')).toHaveValue('');
  await page.getByRole('button', { name: 'Explorer', exact: true }).click();
  await page.getByLabel('Search files').fill('character/mario/P_MR.nsbmd');
  await page.getByRole('treeitem').filter({ hasText: 'P_MR.nsbmd' }).first().dblclick();
  await expect(page.getByLabel('Animation', { exact: true })).toBeEnabled();
  await expect(page.getByRole('button', { name: 'Pause animation' })).toBeVisible();
  await page.getByLabel('Animation', { exact: true }).selectOption('-1');
  await expect(page.getByRole('button', { name: 'Play animation' })).toBeDisabled();
});

test('stale menu files warn and open the copy used by the game', async ({ page }) => {
  await page.goto('/?workspace=Explorer');
  await page.getByLabel('Search files').fill('character/mario/P_MR.nsbmd');
  const row = page.getByTitle('Model: data/KartModelMenu.carc/character/mario/P_MR.nsbmd', {
    exact: true,
  });
  await row.click();
  await expect(page.locator('.warning-banner')).toContainText('stale duplicate');
  await row.dblclick();
  await expect(page.getByLabel('Model file')).toHaveValue(
    'data/KartModelMenu/character/mario/P_MR.nsbmd',
  );
  await expect(page.locator('.warning-banner')).toHaveCount(0);
});

for (const theme of ['dark', 'light']) {
  test(`related pages and Explorer in ${theme}`, async ({ page }) => {
    await page.goto('/?workspace=Related%20assets');
    if (theme === 'light') {
      await page.getByRole('button', { name: 'Settings', exact: true }).click();
      await page.getByLabel('Theme', { exact: true }).selectOption('light');
      await page.getByRole('button', { name: 'Save settings', exact: true }).click();
      await expect(page.getByRole('dialog', { name: 'Settings', exact: true })).toBeHidden();
      await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
    }
    await expect(page.getByLabel('Race driver viewport')).toHaveAttribute('data-loaded', 'true');
    await page.screenshot({
      path: path.resolve(`../docs/screenshots/stage6b-character-${theme}.png`),
    });
    await page.getByRole('button', { name: 'Mario kart 2', exact: true }).first().click();
    await expect(page.getByLabel('Race body viewport')).toHaveAttribute('data-loaded', 'true');
    await page.screenshot({ path: path.resolve(`../docs/screenshots/stage6b-kart-${theme}.png`) });
    await page.getByLabel('Search related assets').fill('Studio name 45');
    await page
      .locator('.related-nav')
      .getByRole('button', { name: 'Studio name 45', exact: true })
      .click();
    await expect(page.getByLabel('Course model viewport')).toHaveAttribute('data-loaded', 'true');
    await page.screenshot({ path: path.resolve(`../docs/screenshots/stage6b-track-${theme}.png`) });
  });
}
