import { test, expect } from '@playwright/test';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';

test.describe('Image Lightbox', () => {
  let tmpDir: string;
  let imagePath: string;

  test.beforeAll(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'playwright-lightbox-'));
    imagePath = path.join(tmpDir, 'test.png');
    fs.writeFileSync(
      imagePath,
      Buffer.from(
        '89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000a49444154789c63000100000500010d0a2db40000000049454e44ae426082',
        'hex'
      )
    );
  });

  test.afterAll(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  async function setupAndUploadImage(page: any, channelName: string, username: string) {
    await page.goto('/register');
    await page.getByTestId('username-input').fill(username);
    await page.getByTestId('password-input').fill('Password123!');
    await page.getByTestId('auth-submit').click();
    await expect(page).toHaveURL(/\/$/);

    const hamburger = page.getByTestId('hamburger-menu');
    const viewport = page.viewportSize();
    if (viewport && viewport.width < 768) {
      await expect(hamburger).toBeVisible();
      await hamburger.click();
    }

    await page.getByTestId('open-create-channel-modal').click();
    await page.getByTestId('new-channel-input').fill(channelName);
    await page.getByTestId('create-channel-button').click();
    await expect(page.getByTestId('chat-header')).toContainText(`# ${channelName}`);

    await page.getByTestId('file-input').setInputFiles(imagePath);
    await expect(page.getByTestId(/pending-attachment-/)).toContainText('test.png');

    await page.getByTestId('chat-input').fill('Test image');
    await page.getByTestId('send-message-button').click();
    await expect(page.getByTestId(/pending-attachment-/)).toHaveCount(0);
  }

  test('Open and close via X button', async ({ page }) => {
    const channelName = `chat_${Math.floor(Math.random() * 1000000)}`;
    const username = `alice_${Math.floor(Math.random() * 1000000)}`;

    await setupAndUploadImage(page, channelName, username);

    const inlineImg = page.getByTestId(/^attachment-img-/).first();
    await expect(inlineImg).toBeVisible();
    await inlineImg.click();

    const overlay = page.getByTestId('image-lightbox-overlay');
    await expect(overlay).toBeVisible();

    const img = page.getByTestId('lightbox-image');
    await expect(img).toBeVisible();

    const closeBtn = page.getByTestId('lightbox-close-button');
    await expect(closeBtn).toBeVisible();
    await closeBtn.click();

    await expect(overlay).not.toBeVisible();
  });

  test('Close via Escape', async ({ page }) => {
    const channelName = `chat_${Math.floor(Math.random() * 1000000)}`;
    const username = `alice_${Math.floor(Math.random() * 1000000)}`;

    await setupAndUploadImage(page, channelName, username);

    const inlineImg = page.getByTestId(/^attachment-img-/).first();
    await expect(inlineImg).toBeVisible();
    await inlineImg.click();

    const overlay = page.getByTestId('image-lightbox-overlay');
    await expect(overlay).toBeVisible();

    await page.keyboard.press('Escape');

    await expect(overlay).not.toBeVisible();
  });

  test('Close via overlay click', async ({ page }) => {
    const channelName = `chat_${Math.floor(Math.random() * 1000000)}`;
    const username = `alice_${Math.floor(Math.random() * 1000000)}`;

    await setupAndUploadImage(page, channelName, username);

    const inlineImg = page.getByTestId(/^attachment-img-/).first();
    await expect(inlineImg).toBeVisible();
    await inlineImg.click();

    const overlay = page.getByTestId('image-lightbox-overlay');
    await expect(overlay).toBeVisible();

    // Click the overlay at a corner so it doesn't hit the container
    await overlay.click({ position: { x: 5, y: 5 } });

    await expect(overlay).not.toBeVisible();
  });

  test('Close via browser back', async ({ page }) => {
    const channelName = `chat_${Math.floor(Math.random() * 1000000)}`;
    const username = `alice_${Math.floor(Math.random() * 1000000)}`;

    await setupAndUploadImage(page, channelName, username);

    const inlineImg = page.getByTestId(/^attachment-img-/).first();
    await expect(inlineImg).toBeVisible();
    await inlineImg.click();

    const overlay = page.getByTestId('image-lightbox-overlay');
    await expect(overlay).toBeVisible();

    await page.goBack();

    await expect(overlay).not.toBeVisible();
  });

  test('Verify mobile viewport behaviour', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });

    const channelName = `chat_${Math.floor(Math.random() * 1000000)}`;
    const username = `alice_${Math.floor(Math.random() * 1000000)}`;

    await setupAndUploadImage(page, channelName, username);

    const inlineImg = page.getByTestId(/^attachment-img-/).first();
    await expect(inlineImg).toBeVisible();
    await inlineImg.click();

    const overlay = page.getByTestId('image-lightbox-overlay');
    await expect(overlay).toBeVisible();

    const closeBtn = page.getByTestId('lightbox-close-button');
    await expect(closeBtn).toBeVisible();
    await closeBtn.click();

    await expect(overlay).not.toBeVisible();
  });
});
