import { test, expect } from '@playwright/test';

test.describe('Channel Management', () => {
  test('user can create and join a channel', async ({ page }) => {
    const username = `user_${Math.floor(Math.random() * 1000000)}`;
    const channelName = `channel_${Math.floor(Math.random() * 1000000)}`;

    // 1. Register and login
    await page.goto('/register');
    await page.getByTestId('username-input').fill(username);
    await page.getByTestId('password-input').fill('Password123!');
    await page.getByTestId('auth-submit').click();
    await expect(page).toHaveURL(/\/$/);

    // 2. Create a channel
    await page.getByTestId('open-create-channel-modal').click();
    await expect(page.getByRole('heading', { name: 'Channels', exact: true })).toBeVisible();
    await page.getByTestId('new-channel-input').fill(channelName);
    await page.getByTestId('create-channel-button').click();

    // 3. Verify channel created and joined (sidebar entry and header)
    await expect(page.getByTestId('channel-list')).toContainText(`# ${channelName}`);
    await expect(page.getByTestId('chat-header')).toContainText(`# ${channelName}`);

    // 4. Test joining an existing channel (from another user's perspective, but here we just browse)
    await page.getByTestId('browse-channels-button').click();
    await expect(page.getByRole('heading', { name: 'Join Existing Channels' })).toBeVisible();
    
    // Search for the channel we just created (it should NOT be there for us since we already joined)
    await page.getByTestId('search-channels-input').fill(channelName);
    await expect(page.getByTestId('no-channels-found')).toBeVisible();

    // Close modal
    await page.getByTestId('close-modal-button').click();
  });
});
