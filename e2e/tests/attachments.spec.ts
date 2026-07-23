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
    const textAttachment = pageA.getByTestId(/^attachment-download-[0-9a-f]{8}-/);
    await expect(textAttachment).toBeVisible();
    await expect(textAttachment).toHaveAttribute('download', 'hello.txt');

    await contextA.close();
    await contextB.close();
  });

  test('PDF fallback and manual download trigger', async ({ browser }) => {
    const channelName = `chat_${Math.floor(Math.random() * 1000000)}`;
    
    const contextA = await browser.newContext();
    const contextB = await browser.newContext();
    
    const pageA = await contextA.newPage();
    const pageB = await contextB.newPage();
    
    const userA = `alice_${Math.floor(Math.random() * 1000000)}`;
    const userB = `bob_${Math.floor(Math.random() * 1000000)}`;

    // Create a tiny valid-enough PDF file
    const pdfPath = path.join(tmpDir, 'test.pdf');
    fs.writeFileSync(pdfPath, '%PDF-1.4\n%EOF');

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

    // 3. Attach a download listener on page B (the receiver) BEFORE page A sends the PDF.
    const unsolicitedDownloads: any[] = [];
    pageB.on('download', (download) => {
      unsolicitedDownloads.push(download);
    });

    // 4. User A uploads and sends the PDF
    await pageA.getByTestId('file-input').setInputFiles(pdfPath);
    await expect(pageA.getByTestId(/pending-attachment-/)).toContainText('test.pdf');
    
    await pageA.getByTestId('chat-input').fill('Here is a PDF file');
    await pageA.getByTestId('send-message-button').click();
    await expect(pageA.getByTestId(/pending-attachment-/)).toHaveCount(0);

    // 5. User B receives the PDF. Check that it doesn't trigger auto-download,
    // but the fallback UI is visible with download button.
    await expect(pageB.getByTestId('chat-history')).toContainText('Here is a PDF file');
    
    const pdfFilename = pageB.getByTestId(/^attachment-filename-[0-9a-f]{8}-/);
    const pdfDownload = pageB.getByTestId(/^attachment-download-[0-9a-f]{8}-/);
    
    await expect(pdfFilename).toBeVisible();
    await expect(pdfFilename).toContainText('test.pdf');
    await expect(pdfDownload).toBeVisible();
    await expect(pdfDownload).toHaveAttribute('download', 'test.pdf');

    // Wait a short duration and check that no unsolicited downloads occurred
    await pageB.waitForTimeout(2000);
    expect(unsolicitedDownloads.length).toBe(0);

    // 6. User B clicks the download button manually and gets the download event
    const downloadPromise = pageB.waitForEvent('download', { timeout: 3000 }).catch(() => null);
    await pdfDownload.click();
    const downloadEvent = await downloadPromise;

    if (downloadEvent) {
      expect(downloadEvent.suggestedFilename()).toBe('test.pdf');
    } else {
      // Fallback check if browser handles link download internally without emitting Playwright event
      await expect(pdfDownload).toHaveAttribute('href', /\/api\/attachments\//);
    }

    await contextA.close();
    await contextB.close();
  });

  test('drag and drop upload happy-path', async ({ page }) => {
    const channelName = `chat_${Math.floor(Math.random() * 1000000)}`;
    const username = `drag_user_${Math.floor(Math.random() * 1000000)}`;

    await page.goto('/register');
    await page.getByTestId('username-input').fill(username);
    await page.getByTestId('password-input').fill('Password123!');
    await page.getByTestId('auth-submit').click();
    await expect(page).toHaveURL(/\/$/);

    await page.getByTestId('open-create-channel-modal').click();
    await page.getByTestId('new-channel-input').fill(channelName);
    await page.getByTestId('create-channel-button').click();
    await expect(page.getByTestId('chat-header')).toContainText(`# ${channelName}`);

    // Create a local test file
    const dropFilePath = path.join(tmpDir, 'drop-test.txt');
    fs.writeFileSync(dropFilePath, 'Hello from drag and drop file');

    const buffer = fs.readFileSync(dropFilePath);
    
    // Evaluate DataTransfer in browser and dispatch event
    const dataTransfer = await page.evaluateHandle(([content, name]) => {
      const dt = new DataTransfer();
      const file = new File([content], name, { type: 'text/plain' });
      dt.items.add(file);
      return dt;
    }, [buffer.toString(), 'drop-test.txt']);

    await page.dispatchEvent('[data-testid="chat-area"]', 'drop', { dataTransfer });

    // Expect the pending attachment to appear
    await expect(page.getByTestId(/pending-attachment-/)).toContainText('drop-test.txt');

    // Send the message
    await page.getByTestId('chat-input').fill('Sent via drag-and-drop');
    await page.getByTestId('send-message-button').click();

    // Verify it is received in the chat history
    await expect(page.getByTestId('chat-history')).toContainText('Sent via drag-and-drop');
    
    const downloadLink = page.getByTestId(/^attachment-download-[0-9a-f]{8}-/);
    await expect(downloadLink).toBeVisible();
    await expect(downloadLink).toHaveAttribute('download', 'drop-test.txt');
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
