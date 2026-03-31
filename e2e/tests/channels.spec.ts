import { test, expect } from '@playwright/test';

test.describe('Channel Management', () => {
  test('user can create and join a channel', async ({ page }) => {
    const username = `user_${Math.floor(Math.random() * 1000000)}`;
    const channelName = `channel_${Math.floor(Math.random() * 1000000)}`;

    // 1. Register and login
    await page.goto('/register');
    await page.getByPlaceholder('Choose a Username').fill(username);
    await page.getByPlaceholder('Choose a Password').fill('Password123!');
    await page.getByRole('button', { name: 'Register' }).click();
    await expect(page).toHaveURL(/\/$/);

    // 2. Create a channel
    await page.locator('.create-button').click();
    await expect(page.getByRole('heading', { name: 'Channels', exact: true })).toBeVisible();
    await page.getByPlaceholder('New channel name...').fill(channelName);
    await page.getByRole('button', { name: 'Create' }).click();

    // 3. Verify channel created and joined (sidebar entry and header)
    await expect(page.locator('.channel-list')).toContainText(`# ${channelName}`);
    await expect(page.locator('.chat-container header')).toContainText(`# ${channelName}`);

    // 4. Test joining an existing channel (from another user's perspective, but here we just browse)
    await page.locator('.browse-button').click();
    await expect(page.getByRole('heading', { name: 'Join Existing Channels' })).toBeVisible();
    
    // Search for the channel we just created (it should NOT be there for us since we already joined)
    await page.getByPlaceholder('Search channels...').fill(channelName);
    await expect(page.getByText('No unjoined channels found')).toBeVisible();

    // Close modal
    await page.locator('.close-button').click();
  });
});
