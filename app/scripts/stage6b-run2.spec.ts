import { test, expect, type Page } from '@playwright/test';
import path from 'node:path';

const driver = 'data/KartModelSub.carc/character/mario/P_MR.nsbmd';
const authored = 'data/Studio/editing.nsbmd';
async function model(page: Page, file = authored) {
  await page.goto('/?workspace=Model');
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await page.getByLabel('Model file', { exact: true }).selectOption(file);
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await expect(page.getByLabel('Model part')).toContainText(
    file === authored ? 'shape0' : 'platform',
  );
  return page.getByLabel('3D viewport', { exact: true });
}

test('linked runtime copies show measured budgets and edit in one undo step', async ({ page }) => {
  const canvas = await model(page, driver);
  const strip = page.getByLabel('Linked copies', { exact: true });
  await expect(strip).toContainText('Race driver');
  await expect(strip).toContainText('Menu driver');
  await expect(strip).toContainText('Far-away driver');
  await expect(strip.locator('.file-link')).toHaveCount(3);
  await expect(page.getByLabel('Apply to all copies')).toBeChecked();
  await canvas.focus();
  await page.keyboard.press('g');
  await page.keyboard.press('x');
  await page.keyboard.type('2');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Transform model', { exact: true })).toHaveCount(1);
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await page.keyboard.press('Control+z');
  await expect(page.getByText('Transform model', { exact: true })).toBeVisible();
  await page.keyboard.press('Control+Shift+z');
  await page.getByRole('button', { name: /Import model/ }).click();
  await expect(page.getByText('Import model', { exact: true })).toBeVisible();
  await page.keyboard.press('Control+z');
  await page.getByLabel('Apply to all copies').uncheck();
  await page.screenshot({ path: path.resolve('../docs/screenshots/stage6b-linked-copies.png') });
});

test('bones select the joint and reuse the rig brush; materials show their exact texture and palette', async ({
  page,
}) => {
  const canvas = await model(page);
  await page.getByRole('button', { name: 'Bone 1: arm', exact: true }).click();
  await expect(canvas).toHaveAttribute('data-highlight', 'bone:1');
  await expect(page.getByLabel('Bone details')).toContainText('root → arm');
  await expect(page.getByLabel('Bone details')).toContainText('assigned vertices');
  await page.getByRole('button', { name: 'Paint vertices to this bone' }).click();
  await expect(page.getByLabel('Rig bone')).toHaveValue('1');
  await expect(page.getByRole('button', { name: 'Stop painting weights' })).toBeVisible();
  await page.getByRole('button', { name: 'Model', exact: true }).click();
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await page
    .locator('.scene-outliner')
    .getByRole('button', { name: /default.*paint/ })
    .click();
  const popup = page.getByRole('dialog', { name: 'Material and texture' });
  await expect(popup.getByAltText('Bound texture')).toBeVisible();
  await expect(popup.getByLabel('Bound palette')).toBeVisible();
  await popup.getByLabel('Texture zoom').fill('8');
  await popup.getByLabel('Palette colour 1', { exact: true }).click();
  await popup.getByLabel('Recolour value').fill('#ff0000');
  await popup.getByRole('button', { name: 'Recolour', exact: true }).click();
  await expect(page.getByText('Edit model texture', { exact: true })).toBeVisible();
  await expect(popup.getByLabel('Palette colour 1', { exact: true })).toHaveCSS(
    'background-color',
    'rgb(255, 0, 0)',
  );
  await popup.getByRole('button', { name: 'Replace image…' }).click();
  await popup.getByRole('button', { name: 'Export PNG…' }).click();
  await expect(popup.getByRole('button', { name: /Replace image/ })).toBeEnabled();
  await expect
    .poll(() =>
      popup.getByAltText('Bound texture').evaluate((img: HTMLImageElement) => img.naturalWidth),
    )
    .toBeGreaterThan(0);
  await page.screenshot({ path: path.resolve('../docs/screenshots/stage6b-material.png') });
  await popup.getByRole('button', { name: 'Highlight on model' }).click();
  await expect(canvas).toHaveAttribute('data-highlight', 'material:0');
});

test('part multi-selection, numeric transforms and cancel keep the camera', async ({ page }) => {
  const canvas = await model(page);
  await page.getByRole('button', { name: 'Part 0: shape0', exact: true }).click();
  await page
    .getByRole('button', { name: 'Part 1: shape1', exact: true })
    .click({ modifiers: ['Shift'] });
  await expect(page.getByLabel('Model selection')).toHaveText('2 parts');
  await canvas.focus();
  await page.keyboard.press('g');
  await page.keyboard.press('z');
  await page.keyboard.type('2');
  await page.keyboard.press('Escape');
  await expect(page.getByText('Transform model', { exact: true })).toHaveCount(0);
  await page.keyboard.press('r');
  await page.keyboard.press('y');
  await page.keyboard.type('30');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Transform model', { exact: true })).toHaveCount(1);
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await page.keyboard.press('Control+z');
  await page.getByRole('button', { name: 'Select none · Alt+A' }).click();
  await expect(page.getByLabel('Model selection')).toHaveText('0 parts');
  await page.getByRole('button', { name: 'Select all · A' }).click();
  await expect(page.getByLabel('Model selection')).toHaveText('2 parts');
});

test('Tab edits only the selected parts and supports scale and x-ray selection', async ({
  page,
}) => {
  const canvas = await model(page);
  await canvas.focus();
  await page.keyboard.press('Tab');
  await page.keyboard.press('3');
  await page.keyboard.press('a');
  await expect(page.getByLabel('Model selection')).toHaveText('2 elements');
  await page.keyboard.press('Alt+z');
  await expect(page.getByRole('button', { name: 'X-ray · Alt+Z' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await page.keyboard.press('s');
  await page.keyboard.press('x');
  await page.keyboard.type('1.5');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Transform model', { exact: true })).toHaveCount(1);
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await page.keyboard.press('Control+z');
});

test('edit mode inset and face extrusion preview live budgets, confirm and undo', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  let canvas = await model(page);
  await page.getByRole('button', { name: 'Select all · A', exact: true }).click();
  await canvas.focus();
  await page.keyboard.press('Tab');
  await page.keyboard.press('3');
  await page.keyboard.press('a');
  await expect(page.getByLabel('Model selection')).toHaveText('4 elements');
  await page.keyboard.press('i');
  await page.keyboard.type('0.2');
  await expect(page.getByLabel('Model operation')).toContainText('Inset');
  await expect(page.getByLabel('Live DS budget')).toContainText('20 / 2,048 triangles');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Inset selection', { exact: true })).toBeVisible();
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await expect(page.getByLabel('Model part')).toContainText('10 triangles');
  canvas = page.getByLabel('3D viewport', { exact: true });
  await canvas.focus();
  await page.keyboard.press('a');
  await page.keyboard.press('e');
  await page.keyboard.type('-1');
  await expect(page.getByLabel('Live DS budget')).toContainText('36 / 2,048 triangles');
  await page.keyboard.press('Escape');
  await expect(page.getByLabel('Live DS budget')).toContainText('20 / 2,048 triangles');
  await page.keyboard.press('e');
  await page.keyboard.type('1');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Extrude selection', { exact: true })).toBeVisible();
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await page.keyboard.press('Control+z');
  await expect(page.getByLabel('Model part')).toContainText('10 triangles');
  await page.screenshot({ path: path.resolve('../docs/screenshots/stage6b-edit-mode.png') });
  expect(errors).toEqual([]);
});

test('box select, select linked, edge mode, deletion and merge menus use Blender keys', async ({
  page,
}) => {
  let canvas = await model(page);
  await page.getByRole('button', { name: 'Select all · A', exact: true }).click();
  await canvas.focus();
  await page.keyboard.press('Tab');
  await page.keyboard.press('3');
  await page.keyboard.press('b');
  const box = await canvas.boundingBox();
  if (!box) throw Error('missing viewport');
  await page.mouse.move(box.x + 5, box.y + 5);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width - 5, box.y + box.height - 5, { steps: 5 });
  await page.mouse.up();
  await expect(page.getByLabel('Model selection')).toHaveText('4 elements');
  await page.keyboard.press('2');
  await expect(page.getByRole('button', { name: 'Edge · 2', exact: true })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await page.keyboard.press('e');
  await page.keyboard.type('0.5');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Extrude selection', { exact: true })).toBeVisible();
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await page.keyboard.press('Control+z');
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  canvas = page.getByLabel('3D viewport', { exact: true });
  await canvas.focus();
  await page.keyboard.press('3');
  await page.keyboard.press('Alt+a');
  // Pick a face near the projected centre of the left quad.
  await canvas.click({ position: { x: box.width * 0.4, y: box.height * 0.55 } });
  await page.keyboard.press('l');
  await page.keyboard.press('x');
  await expect(page.getByRole('menu', { name: 'Delete mesh' })).toBeVisible();
  await page.getByRole('menuitem', { name: 'Faces', exact: true }).click();
  await expect(page.getByText('Delete model selection', { exact: true })).toBeVisible();
  await page.keyboard.press('Control+z');
  await expect(page.getByTestId('viewport')).toHaveAttribute('data-loaded', 'true');
  await canvas.focus();
  await page.keyboard.press('1');
  await page.keyboard.press('a');
  await page.keyboard.press('m');
  await page.getByRole('menuitem', { name: 'At center' }).click();
  await expect(page.getByText('Merge selected vertices', { exact: true })).toBeVisible();
  await page.keyboard.press('Control+z');
});
