import { test, expect } from '@playwright/test';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';

test.describe('File Attachments', () => {
  let tmpDir: string;

  test.beforeAll(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'playwright-test-'));
  });

  test.afterAll(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test('upload and receive attachments', async ({ browser }) => {
    const channelName = `chat_${Math.floor(Math.random() * 1000000)}`;
    
    const contextA = await browser.newContext();
    const contextB = await browser.newContext();
    
    const pageA = await contextA.newPage();
    const pageB = await contextB.newPage();
    
    const userA = `alice_${Math.floor(Math.random() * 1000000)}`;
    const userB = `bob_${Math.floor(Math.random() * 1000000)}`;

    // Create test files
    const imagePath = path.join(tmpDir, 'test.png');
    fs.writeFileSync(imagePath, Buffer.from('89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000a49444154789c63000100000500010d0a2db40000000049454e44ae426082', 'hex'));
    
    const textPath = path.join(tmpDir, 'hello.txt');
    fs.writeFileSync(textPath, 'Hello from text file');

    // 1. User A registers and creates channel
    await pageA.goto('/register');
    await pageA.getByTestId('username-input').fill(userA);
    await pageA.getByTestId('password-input').fill('Password123!');
    await pageA.getByTestId('auth-submit').click();
    await expect(pageA).toHaveURL(/\/$/);
    
    await pageA.getByTestId('open-create-channel-modal').click();
    await pageA.getByTestId('new-channel-input').fill(channelName);
    await pageA.getByTestId('create-channel-button').click();
    await expect(pageA.getByTestId('chat-header')).toContainText(`# ${channelName}`);

    // 2. User B registers and joins channel
    await pageB.goto('/register');
    await pageB.getByTestId('username-input').fill(userB);
    await pageB.getByTestId('password-input').fill('Password123!');
    await pageB.getByTestId('auth-submit').click();
    await expect(pageB).toHaveURL(/\/$/);
    
    const responsePromise = pageB.waitForResponse(r => r.url().includes('/api/channels/unjoined'));
    await pageB.getByTestId('browse-channels-button').click();
    await responsePromise;

    await pageB.getByTestId('search-channels-input').fill(channelName);
    await pageB.getByTestId('unjoined-channel-item').filter({ hasText: channelName }).getByTestId('join-channel-button').click();
    await expect(pageB.getByTestId('chat-header')).toContainText(`# ${channelName}`);

    // 3. User A uploads an image
    await pageA.getByTestId('file-input').setInputFiles(imagePath);
    await expect(pageA.getByTestId(/pending-attachment-/)).toContainText('test.png');
    
    await pageA.getByTestId('chat-input').fill('Here is an image');
    await pageA.getByTestId('send-message-button').click();
    
    // Ensure pending attachment disappears after sending
    await expect(pageA.getByTestId(/pending-attachment-/)).toHaveCount(0);

    // 4. User B receives the image
    await expect(pageB.getByTestId('chat-history')).toContainText('Here is an image');
    const imageAttachment = pageB.getByTestId(/^attachment-[0-9a-f]{8}-/).locator('img[src^="/api/attachments/"]');
    await expect(imageAttachment).toBeVisible();

    // 5. User B uploads a text file
    await pageB.getByTestId('file-input').setInputFiles(textPath);
    await expect(pageB.getByTestId(/pending-attachment-/)).toContainText('hello.txt');

    await pageB.getByTestId('chat-input').fill('Here is a document');
    await pageB.getByTestId('send-message-button').click();

    // 6. User A receives the text file as a download link
    await expect(pageA.getByTestId('chat-history')).toContainText('Here is a document');
    const textAttachment = pageA.getByTestId(/^attachment-[0-9a-f]{8}-/).locator('a[download="hello.txt"]');
    await expect(textAttachment).toBeVisible();

    await contextA.close();
    await contextB.close();
  });

  test('upload exceeds size limit', async ({ page }) => {
    // Generate a 50.1 MB dummy file
    const largeFilePath = path.join(tmpDir, 'large.bin');
    const buffer = Buffer.alloc(50 * 1024 * 1024 + 1024); // slightly over 50MB
    fs.writeFileSync(largeFilePath, buffer);

    await page.goto('/register');
    const user = `carl_${Math.floor(Math.random() * 1000000)}`;
    await page.getByTestId('username-input').fill(user);
    await page.getByTestId('password-input').fill('Password123!');
    await page.getByTestId('auth-submit').click();
    await expect(page).toHaveURL(/\/$/);

    // Attempt to upload large file
    await page.getByTestId('file-input').setInputFiles(largeFilePath);

    // Expect the error message to appear
    await expect(page.getByTestId('attachment-error')).toContainText('exceeds 50MB limit');
    await expect(page.getByTestId(/pending-attachment-/)).toHaveCount(0);
  });
});
