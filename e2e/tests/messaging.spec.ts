import { test, expect } from '@playwright/test';

test.describe('Real-time Messaging', () => {
  test('two users can communicate in a channel', async ({ browser }) => {
    const channelName = `chat_${Math.floor(Math.random() * 1000000)}`;
    
    // Create two separate browser contexts to simulate two different users
    const contextA = await browser.newContext();
    const contextB = await browser.newContext();
    
    const pageA = await contextA.newPage();
    const pageB = await contextB.newPage();
    
    const userA = `alice_${Math.floor(Math.random() * 1000000)}`;
    const userB = `bob_${Math.floor(Math.random() * 1000000)}`;

    // 1. User A registers and creates a channel
    await pageA.goto('/register');
    await pageA.getByPlaceholder('Choose a Username').fill(userA);
    await pageA.getByPlaceholder('Choose a Password').fill('Password123!');
    await pageA.getByRole('button', { name: 'Register' }).click();
    await expect(pageA).toHaveURL(/\/$/);
    
    await pageA.locator('.create-button').click();
    await pageA.getByPlaceholder('New channel name...').fill(channelName);
    await pageA.getByRole('button', { name: 'Create' }).click();
    await expect(pageA.locator('.chat-container header h1')).toContainText(`# ${channelName}`);

    // 2. User B registers and joins the same channel
    await pageB.goto('/register');
    await pageB.getByPlaceholder('Choose a Username').fill(userB);
    await pageB.getByPlaceholder('Choose a Password').fill('Password123!');
    await pageB.getByRole('button', { name: 'Register' }).click();
    await expect(pageB).toHaveURL(/\/$/);
    
    await pageB.locator('.browse-button').click();
    await pageB.getByPlaceholder('Search channels...').fill(channelName);
    await pageB.getByRole('button', { name: 'Join' }).click();
    await expect(pageB.locator('.chat-container header h1')).toContainText(`# ${channelName}`);

    // 3. User A sends a message
    const messageText = `Hello from Alice at ${new Date().toISOString()}`;
    await pageA.getByPlaceholder(`Message #${channelName}`).fill(messageText);
    await pageA.getByRole('button', { name: 'Send' }).click();

    // 4. User B should receive the message in real-time
    await expect(pageB.locator('.chat-history')).toContainText(messageText);
    await expect(pageB.locator('.chat-history')).toContainText(userA);

    // 5. User B replies
    const replyText = `Hi Alice! Bob here.`;
    await pageB.getByPlaceholder(`Message #${channelName}`).fill(replyText);
    await pageB.getByRole('button', { name: 'Send' }).click();

    // 6. User A should receive the reply
    await expect(pageA.locator('.chat-history')).toContainText(replyText);
    await expect(pageA.locator('.chat-history')).toContainText(userB);

    await contextA.close();
    await contextB.close();
  });
});
