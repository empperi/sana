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
    await page.getByTestId('username-input').fill(username);
    await page.getByTestId('password-input').fill(password);
    await page.getByTestId('auth-submit').click();

    // After registration, it should redirect to chat (/)
    await expect(page).toHaveURL(/\/$/);
    
    // Verify we are in the chat app
    await expect(page.getByTestId('profile-button')).toBeVisible();
    await expect(page.getByTestId('profile-button')).toContainText(username);
  });

  test('user can login and logout', async ({ page }) => {
    const username = `user_${Math.floor(Math.random() * 1000000)}`;
    const password = 'Password123!';

    // 1. Register
    await page.goto('/register');
    await page.getByTestId('username-input').fill(username);
    await page.getByTestId('password-input').fill(password);
    await page.getByTestId('auth-submit').click();
    await expect(page).toHaveURL(/\/$/);

    // 2. Logout
    await page.getByTestId('profile-button').click();
    await page.getByTestId('logout-button').click();
    await expect(page).toHaveURL(/\/login/);

    // 3. Login
    await page.getByTestId('username-input').fill(username);
    await page.getByTestId('password-input').fill(password);
    await page.getByTestId('auth-submit').click();
    await expect(page).toHaveURL(/\/$/);

    // 4. Verify session persistence (reload)
    await page.reload();
    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByTestId('sidebar')).toBeVisible();

    // 5. Logout again
    await page.getByTestId('profile-button').click();
    await page.getByTestId('logout-button').click();
    await expect(page).toHaveURL(/\/login/);
  });
});
