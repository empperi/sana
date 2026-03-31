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
    await pageA.getByTestId('username-input').fill(userA);
    await pageA.getByTestId('password-input').fill('Password123!');
    await pageA.getByTestId('auth-submit').click();
    await expect(pageA).toHaveURL(/\/$/);
    
    await pageA.getByTestId('open-create-channel-modal').click();
    await pageA.getByTestId('new-channel-input').fill(channelName);
    await pageA.getByTestId('create-channel-button').click();
    await expect(pageA.getByTestId('chat-header')).toContainText(`# ${channelName}`);

    // 2. User B registers and joins the same channel
    await pageB.goto('/register');
    await pageB.getByTestId('username-input').fill(userB);
    await pageB.getByTestId('password-input').fill('Password123!');
    await pageB.getByTestId('auth-submit').click();
    await expect(pageB).toHaveURL(/\/$/);
    
    await pageB.getByTestId('browse-channels-button').click();
    await pageB.getByTestId('search-channels-input').fill(channelName);
    await pageB.getByTestId('join-channel-button').click();
    await expect(pageB.getByTestId('chat-header')).toContainText(`# ${channelName}`);

    // 3. User A sends a message
    const messageText = `Hello from Alice at ${new Date().toISOString()}`;
    await pageA.getByTestId('chat-input').fill(messageText);
    await pageA.getByTestId('send-message-button').click();

    // 4. User B should receive the message in real-time
    await expect(pageB.getByTestId('chat-history')).toContainText(messageText);
    await expect(pageB.getByTestId('chat-history')).toContainText(userA);

    // 5. User B replies
    const replyText = `Hi Alice! Bob here.`;
    await pageB.getByTestId('chat-input').fill(replyText);
    await pageB.getByTestId('send-message-button').click();

    // 6. User A should receive the reply
    await expect(pageA.getByTestId('chat-history')).toContainText(replyText);
    await expect(pageA.getByTestId('chat-history')).toContainText(userB);

    await contextA.close();
    await contextB.close();
  });
});
