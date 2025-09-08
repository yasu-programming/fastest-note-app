import { test, expect } from '@playwright/test';

test.describe('Authentication', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to auth page before each test
    await page.goto('/auth');
  });

  test('should display login form by default', async ({ page }) => {
    await expect(page.locator('h2')).toContainText('Sign In');
    await expect(page.getByPlaceholder('your@email.com')).toBeVisible();
    await expect(page.getByPlaceholder('Enter your password')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Sign In' })).toBeVisible();
  });

  test('should switch to register form', async ({ page }) => {
    await page.getByRole('button', { name: 'Sign up here' }).click();
    
    await expect(page.locator('h2')).toContainText('Create Account');
    await expect(page.getByPlaceholder('your@email.com')).toBeVisible();
    await expect(page.getByPlaceholder('Create a strong password')).toBeVisible();
    await expect(page.getByPlaceholder('Confirm your password')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Create Account' })).toBeVisible();
  });

  test('should show validation errors for invalid login', async ({ page }) => {
    // Enter invalid email
    await page.getByPlaceholder('your@email.com').fill('invalid-email');
    await page.getByRole('button', { name: 'Sign In' }).click();
    
    await expect(page.getByText('Please enter a valid email address')).toBeVisible();
  });

  test('should show password requirements in register form', async ({ page }) => {
    await page.getByRole('button', { name: 'Sign up here' }).click();
    
    await expect(page.getByText('Password Requirements:')).toBeVisible();
    await expect(page.getByText('At least 8 characters long')).toBeVisible();
    await expect(page.getByText('One lowercase letter')).toBeVisible();
    await expect(page.getByText('One uppercase letter')).toBeVisible();
    await expect(page.getByText('One number')).toBeVisible();
    await expect(page.getByText('One special character')).toBeVisible();
  });

  test('should show password strength indicator', async ({ page }) => {
    await page.getByRole('button', { name: 'Sign up here' }).click();
    
    const passwordInput = page.getByPlaceholder('Create a strong password');
    
    // Type weak password
    await passwordInput.fill('weak');
    await expect(page.getByText('Very Weak')).toBeVisible();
    
    // Type strong password
    await passwordInput.fill('StrongPassword123!');
    await expect(page.getByText('Strong')).toBeVisible();
  });

  test('should validate password confirmation', async ({ page }) => {
    await page.getByRole('button', { name: 'Sign up here' }).click();
    
    await page.getByPlaceholder('Create a strong password').fill('StrongPassword123!');
    await page.getByPlaceholder('Confirm your password').fill('DifferentPassword123!');
    await page.getByRole('button', { name: 'Create Account' }).click();
    
    await expect(page.getByText('Passwords do not match')).toBeVisible();
  });

  test('should attempt login with valid credentials', async ({ page }) => {
    // Mock the API response for successful login
    await page.route('**/auth/login', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user: {
            id: '1',
            email: 'test@example.com',
            created_at: '2024-01-01T00:00:00Z',
            updated_at: '2024-01-01T00:00:00Z',
          },
          access_token: 'mock-access-token',
          refresh_token: 'mock-refresh-token',
          expires_in: 3600,
        }),
      });
    });

    await page.getByPlaceholder('your@email.com').fill('test@example.com');
    await page.getByPlaceholder('Enter your password').fill('password123');
    await page.getByRole('button', { name: 'Sign In' }).click();

    // Should redirect to dashboard on successful login
    await expect(page).toHaveURL('/dashboard');
  });

  test('should show error for invalid credentials', async ({ page }) => {
    // Mock API response for failed login
    await page.route('**/auth/login', async (route) => {
      await route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({
          error: 'Invalid credentials',
        }),
      });
    });

    await page.getByPlaceholder('your@email.com').fill('test@example.com');
    await page.getByPlaceholder('Enter your password').fill('wrongpassword');
    await page.getByRole('button', { name: 'Sign In' }).click();

    await expect(page.getByText('Invalid credentials')).toBeVisible();
  });

  test('should attempt registration with valid data', async ({ page }) => {
    // Mock successful registration
    await page.route('**/auth/register', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user: {
            id: '1',
            email: 'newuser@example.com',
            created_at: '2024-01-01T00:00:00Z',
            updated_at: '2024-01-01T00:00:00Z',
          },
          access_token: 'mock-access-token',
          refresh_token: 'mock-refresh-token',
          expires_in: 3600,
        }),
      });
    });

    await page.getByRole('button', { name: 'Sign up here' }).click();
    
    await page.getByPlaceholder('your@email.com').fill('newuser@example.com');
    await page.getByPlaceholder('Create a strong password').fill('NewUserPass123!');
    await page.getByPlaceholder('Confirm your password').fill('NewUserPass123!');
    await page.getByRole('button', { name: 'Create Account' }).click();

    // Should redirect to dashboard on successful registration
    await expect(page).toHaveURL('/dashboard');
  });

  test('should show loading state during authentication', async ({ page }) => {
    // Mock slow API response
    await page.route('**/auth/login', async (route) => {
      await new Promise(resolve => setTimeout(resolve, 1000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          user: { id: '1', email: 'test@example.com' },
          access_token: 'token',
          refresh_token: 'refresh',
          expires_in: 3600,
        }),
      });
    });

    await page.getByPlaceholder('your@email.com').fill('test@example.com');
    await page.getByPlaceholder('Enter your password').fill('password123');
    
    const submitButton = page.getByRole('button', { name: 'Sign In' });
    await submitButton.click();

    // Should show loading state
    await expect(page.getByText('Signing In...')).toBeVisible();
    await expect(submitButton).toBeDisabled();

    // Wait for completion
    await expect(page).toHaveURL('/dashboard');
  });

  test('should display app branding and features', async ({ page }) => {
    await expect(page.getByText('Fastest Note App')).toBeVisible();
    await expect(page.getByText('Lightning-fast note-taking with real-time sync')).toBeVisible();
    
    // Feature highlights
    await expect(page.getByText('Lightning Fast')).toBeVisible();
    await expect(page.getByText('Real-time Sync')).toBeVisible();
    await expect(page.getByText('Organized')).toBeVisible();
    
    // Demo access
    await expect(page.getByText('Try Demo Version (No Account Required)')).toBeVisible();
  });

  test('should handle keyboard navigation', async ({ page }) => {
    const emailInput = page.getByPlaceholder('your@email.com');
    const passwordInput = page.getByPlaceholder('Enter your password');
    const submitButton = page.getByRole('button', { name: 'Sign In' });

    // Tab navigation
    await emailInput.focus();
    await page.keyboard.press('Tab');
    await expect(passwordInput).toBeFocused();
    
    await page.keyboard.press('Tab');
    await expect(submitButton).toBeFocused();

    // Enter key should submit form
    await emailInput.fill('test@example.com');
    await passwordInput.fill('password123');
    await passwordInput.press('Enter');

    // Form should be submitted (would show validation or loading)
    await expect(page.getByText('Email is required').or(page.getByText('Signing In...'))).toBeVisible();
  });
});

test.describe('Authentication Accessibility', () => {
  test('should be accessible', async ({ page }) => {
    await page.goto('/auth');

    // Check for proper form labels
    await expect(page.getByLabel('Email Address')).toBeVisible();
    await expect(page.getByLabel('Password')).toBeVisible();

    // Check for proper heading structure
    const headings = await page.locator('h1, h2, h3').all();
    expect(headings.length).toBeGreaterThan(0);

    // Check for proper button roles
    const buttons = await page.getByRole('button').all();
    expect(buttons.length).toBeGreaterThan(0);

    // Check for proper input types
    await expect(page.getByRole('textbox', { name: 'Email Address' })).toHaveAttribute('type', 'email');
    await expect(page.locator('input[type="password"]')).toBeVisible();
  });

  test('should support screen reader navigation', async ({ page }) => {
    await page.goto('/auth');

    // Check for proper ARIA attributes
    const form = page.locator('form');
    await expect(form).toBeVisible();

    // Error messages should be associated with inputs
    await page.getByPlaceholder('your@email.com').fill('invalid');
    await page.getByRole('button', { name: 'Sign In' }).click();

    const errorMessage = page.getByText('Please enter a valid email address');
    await expect(errorMessage).toBeVisible();
    await expect(errorMessage).toHaveAttribute('role', 'alert');
  });
});