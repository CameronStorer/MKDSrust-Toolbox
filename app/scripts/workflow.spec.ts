import { test, expect } from '@playwright/test';
test('palette history, immutable text codes, settings and packaging', async ({ page }) => {
  await page.goto('/?workspace=Overview');
  await page.getByRole('button', { name: 'Texture & 2D', exact: true }).click();
  await page.getByRole('button', { name: 'Colour 1', exact: true }).click();
  const before = await page.getByLabel('R channel').inputValue();
  await page.getByLabel('R channel').fill(before === '31' ? '30' : '31');
  await expect(page.getByLabel('R channel')).not.toHaveValue(before);
  await page.keyboard.press('Control+z');
  await expect(page.getByLabel('R channel')).toHaveValue(before);
  await page.keyboard.press('Control+Shift+z');
  await expect(page.getByLabel('R channel')).not.toHaveValue(before);
  await page.getByRole('button', { name: 'Text', exact: true }).click();
  await page.getByLabel('BMG file').selectOption('data/Studio/demo.carc/language/en.bmg');
  await page.getByRole('button', { name: /0001.*Take the/ }).click();
  await expect(page.getByText('Game control 1', { exact: true })).toBeVisible();
  await page.getByLabel('Text segment 1').fill('Travel the ');
  await page.getByRole('button', { name: 'Save message', exact: true }).click();
  await expect(page.getByText('Game control 1', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Overview', exact: true }).click();
  await page.getByRole('button', { name: 'Pack mod', exact: true }).click();
  await expect(page.getByText('Package built', { exact: false })).toBeVisible();
  await page.keyboard.press('F3');
  await page.getByLabel('Search commands').fill('settings');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('dialog', { name: 'Settings', exact: true })).toBeVisible();
});
test('layout persistence and reset', async ({ page }) => {
  await page.goto('/?workspace=Explorer');
  await expect(page.getByText('Hex view', { exact: true })).toBeVisible();
  await page.waitForFunction(() => !!localStorage.getItem('layout:Explorer'));
  const saved = await page.evaluate(() => localStorage.getItem('layout:Explorer'));
  await page.reload();
  await expect(page.getByText('Hex view', { exact: true })).toBeVisible();
  expect(JSON.parse((await page.evaluate(() => localStorage.getItem('layout:Explorer')))!)).toEqual(
    JSON.parse(saved!),
  );
  await page.keyboard.press('F3');
  await page.getByLabel('Search commands').fill('reset workspace');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Hex view', { exact: true })).toBeVisible();
});

test('palette render changes and undo restores its pixels', async ({ page }) => {
  await page.goto('/?workspace=Overview');
  await page.getByRole('button', { name: 'Texture & 2D', exact: true }).click();
  const canvas = page.getByLabel('Texture preview');
  await expect(canvas).toHaveAttribute('width', '128');
  const pixel = () =>
    canvas.evaluate((element) =>
      Array.from((element as HTMLCanvasElement).getContext('2d')!.getImageData(0, 0, 1, 1).data),
    );
  const original = await pixel();
  await page.getByRole('button', { name: 'Colour 1', exact: true }).click();
  await page.getByLabel('R channel').fill(original[0] === 255 ? '0' : '31');
  await expect.poll(pixel).not.toEqual(original);
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect.poll(pixel).toEqual(original);
});
test('text revert restores the message and can be undone', async ({ page }) => {
  await page.goto('/?workspace=Overview');
  await page.getByRole('button', { name: 'Text', exact: true }).click();
  const path = 'data/Studio/demo.carc/language/en.bmg';
  await page.getByLabel('BMG file').selectOption(path);
  await page.getByLabel('Text segment 1').fill('An authored replacement.');
  await page.getByRole('button', { name: 'Save message', exact: true }).click();
  await page.getByRole('button', { name: 'Overview', exact: true }).click();
  await page.getByRole('button', { name: 'Revert ' + path, exact: true }).click();
  await page.getByRole('button', { name: 'Text', exact: true }).click();
  await expect(page.getByLabel('Text segment 1')).toHaveValue('Welcome to your next idea.');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.getByLabel('Text segment 1')).toHaveValue('An authored replacement.');
});
test('first run opens a ROM before creating a project', async ({ page }) => {
  await page.goto('/?onboarding=1');
  await expect(page.getByRole('button', { name: /^Replace a character/ })).toBeDisabled();
  await page.getByRole('button', { name: 'Open ROM…', exact: true }).click();
  await page.getByRole('button', { name: 'Create project…', exact: true }).click();
  await page.getByRole('button', { name: 'Choose folder…', exact: true }).click();
  await page
    .getByRole('dialog', { name: 'New project' })
    .getByRole('button', { name: 'Create project', exact: true })
    .click();
  await expect(page.getByRole('heading', { name: 'What do you want to make?' })).toBeVisible();
  await page.getByRole('button', { name: /^Change text and names/ }).click();
  await expect(page.getByLabel('Text segment 1')).toBeEnabled();
});
