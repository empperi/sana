import { test, expect } from '@playwright/test';

test.describe('Authentication', () => {
  test('user can register', async ({ page }) => {
    const username = `user_${Math.floor(Math.random() * 1000000)}`;
    const password = 'Password123!';

    await page.goto('/');
    
    // Should be redirected to login
    await expect(page).toHaveURL(/\/login/);
    
    // Click on "Create one" to go to registration
    await page.getByRole('link', { name: 'Create one' }).click();
    await expect(page).toHaveURL(/\/register/);

    // Fill registration form
    await page.getByPlaceholder('Choose a Username').fill(username);
    await page.getByPlaceholder('Choose a Password').fill(password);
    await page.getByRole('button', { name: 'Register' }).click();

    // After registration, it should redirect to chat (/)
    await expect(page).toHaveURL(/\/$/);
    
    // Verify we are in the chat app by checking for the sidebar or chat window
    await expect(page.locator('.app-container')).toBeVisible();
    await expect(page.getByText(username)).toBeVisible();
  });

  test('user can login and logout', async ({ page }) => {
    const username = `user_${Math.floor(Math.random() * 1000000)}`;
    const password = 'Password123!';

    // 1. Register
    await page.goto('/register');
    await page.getByPlaceholder('Choose a Username').fill(username);
    await page.getByPlaceholder('Choose a Password').fill(password);
    await page.getByRole('button', { name: 'Register' }).click();
    await expect(page).toHaveURL(/\/$/);

    // 2. Logout
    await page.locator('.profile-button').click();
    await page.getByRole('button', { name: 'Logout' }).click();
    await expect(page).toHaveURL(/\/login/);

    // 3. Login
    await page.getByPlaceholder('Username').fill(username);
    await page.getByPlaceholder('Password').fill(password);
    await page.getByRole('button', { name: 'Login' }).click();
    await expect(page).toHaveURL(/\/$/);

    // 4. Verify session persistence (reload)
    await page.reload();
    await expect(page).toHaveURL(/\/$/);
    await expect(page.locator('.app-container')).toBeVisible();

    // 5. Logout again
    await page.locator('.profile-button').click();
    await page.getByRole('button', { name: 'Logout' }).click();
    await expect(page).toHaveURL(/\/login/);
  });
});
