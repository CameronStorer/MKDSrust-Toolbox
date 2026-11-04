import { test, expect } from '@playwright/test';
import path from 'node:path';

for (const [card, flow] of [
  ['Replace a character', 'Character'],
  ['Replace a kart', 'Kart'],
  ['Make a track', 'Track'],
  ['Make a cup', 'Cups'],
]) {
  test(`Home card ${card} applies and saves`, async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'What do you want to make?' })).toBeVisible();
    await page.getByRole('button', { name: new RegExp(`^${card}`) }).click();
    const dialog = page.getByRole('dialog', { name: 'Beginner recipes' });
    if (flow !== 'Cups') await dialog.getByLabel('Recipe source').fill('/author/generated.glb');
    await dialog.getByRole('button', { name: `Apply ${flow.toLowerCase()}`, exact: true }).click();
    await expect(dialog.getByText(/Applied/)).toBeVisible();
    await page.getByLabel('Close recipes').click();
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(page.locator('.save-state')).toHaveText('Saved');
  });
}
test('Text card defaults to English US, keeps drafts across tabs and Ctrl S saves', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: /^Change text and names/ }).click();
  await expect(page.getByLabel('Text language')).toHaveValue('English (US)');
  await expect(page.getByLabel('BMG file')).toHaveValue('data/Main2D_us.carc/common.bmg');
  await page.getByLabel('Text segment 1').fill('Authored menu label');
  await expect(page.locator('.save-state')).toHaveText('Unsaved changes');
  await page.getByRole('button', { name: 'Home', exact: true }).click();
  await page.getByRole('button', { name: /^Change text and names/ }).click();
  await expect(page.getByLabel('Text segment 1')).toHaveValue('Authored menu label');
  await page.getByLabel('Text segment 1').focus();
  await page.keyboard.press('Control+s');
  await expect(page.locator('.save-state')).toHaveText('Saved');
  await page.getByLabel('Text language').selectOption('Spanish');
  await expect(page.getByLabel('BMG file')).toHaveValue('data/Main2D_es.carc/common.bmg');
  await expect(page.getByLabel('Text segment 1')).toHaveValue('Prueba 0');
  await page.getByLabel('Text language').selectOption('English (US)');
  await expect(page.getByLabel('Text segment 1')).toHaveValue('Authored menu label');
  await page.getByLabel('Search all text and languages').check();
  await page.getByLabel('Search messages').fill('Authored menu label');
  await expect(page.getByRole('button', { name: /Authored menu label/ })).toBeVisible();
  await page.screenshot({
    path: path.resolve('../docs/screenshots/stage6-text.png'),
    fullPage: true,
  });
});
test('Game browser explains race and far-away models, tabs and back forward', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto('/');
  await page.getByRole('button', { name: 'Game browser', exact: true }).click();
  await page.getByLabel('Search game').fill('Mario race model');
  await page
    .locator('.game-card')
    .filter({ has: page.getByText('Mario race model', { exact: true }) })
    .first()
    .click();
  await expect(page.getByRole('tab', { name: 'Mario race model' })).toBeVisible();
  await page.getByLabel('Search game').fill('Mario far-away model');
  await page
    .locator('.game-card')
    .filter({ has: page.getByText('Mario far-away model', { exact: true }) })
    .first()
    .click();
  await expect(page.locator('.game-preview').getByText(/opponent at a distance/)).toBeVisible();
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await expect(page.getByRole('tab', { name: 'Mario race model' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  await page.getByRole('button', { name: 'Forward', exact: true }).click();
  await expect(page.getByRole('tab', { name: 'Mario far-away model' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  await page.screenshot({
    path: path.resolve('../docs/screenshots/stage6-game-browser.png'),
    fullPage: true,
  });
  expect(errors).toEqual([]);
});
test('Image card swaps a colour with automatic pairing and previews PNG import', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: /^Recolour or redraw/ }).click();
  await page.getByLabel('Search game').fill('art/studio.ncgr');
  await page
    .locator('.game-card')
    .filter({ has: page.getByText('studio', { exact: true }) })
    .first()
    .click();
  await expect(page.getByLabel('Texture preview')).toHaveAttribute('width', '128');
  await expect(page.getByRole('heading', { name: 'Swap a colour' })).toBeVisible();
  await expect(page.getByText(/This palette is also used by/)).toBeVisible();
  await page.getByRole('button', { name: 'Colour 1', exact: true }).click();
  await page.getByLabel('Colour picker').fill('#ff0000');
  await page.getByRole('button', { name: 'Paint individual tiles' }).click();
  await page.getByRole('button', { name: 'Import PNG…', exact: true }).click();
  await page.getByRole('button', { name: 'Preview conversion' }).click();
  await expect(page.getByAltText('After conversion')).toBeVisible();
  await page.getByRole('button', { name: 'Apply PNG import', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Apply PNG import', exact: true })).toBeHidden();
  await page.screenshot({
    path: path.resolve('../docs/screenshots/stage6-image-gallery.png'),
    fullPage: true,
  });
});
test('Music card plays audible media with duration and stops, then imports a song', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: /^Change music/ }).click();
  await page.getByRole('button', { name: 'Play preview', exact: true }).click();
  const audio = page.getByLabel('Audio preview');
  await expect
    .poll(() => audio.evaluate((e) => (e as HTMLAudioElement).duration))
    .toBeGreaterThan(0);
  await expect.poll(() => audio.evaluate((e) => (e as HTMLAudioElement).paused)).toBe(false);
  await page.getByRole('button', { name: 'Stop preview' }).click();
  await expect.poll(() => audio.evaluate((e) => (e as HTMLAudioElement).paused)).toBe(true);
  await page.getByRole('button', { name: 'Replace this song' }).click();
  await page.getByRole('button', { name: 'Choose WAV / MIDI…' }).click();
  await page.getByRole('button', { name: 'Import song', exact: true }).click();
  await expect(page.getByText(/Imported sampled/)).toBeVisible();
  await page.screenshot({
    path: path.resolve('../docs/screenshots/stage6-audio.png'),
    fullPage: true,
  });
});
test('Home, wizard and Advanced remain reachable', async ({ page }) => {
  await page.goto('/');
  await page.screenshot({
    path: path.resolve('../docs/screenshots/stage6-home.png'),
    fullPage: true,
  });
  await expect(page.getByRole('button', { name: 'Explorer', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Advanced', exact: true }).click();
  await page.getByRole('button', { name: 'Explorer', exact: true }).click();
  await expect(page.getByText('Hex view', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Home', exact: true }).click();
  await page.getByRole('button', { name: /^Replace a character/ }).click();
  await page.screenshot({
    path: path.resolve('../docs/screenshots/stage6-wizard.png'),
    fullPage: true,
  });
});
