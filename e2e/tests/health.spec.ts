import { test, expect } from '@playwright/test';

test('has title', async ({ page }) => {
  await page.goto('/');

  // Expect a title "to contain" a substring.
  await expect(page).toHaveTitle(/Sana/);
});

test('shows login form', async ({ page }) => {
  await page.goto('/');

  // Expect the login form title to be visible.
  await expect(page.getByRole('heading', { name: 'Login to Sana' })).toBeVisible();
});
