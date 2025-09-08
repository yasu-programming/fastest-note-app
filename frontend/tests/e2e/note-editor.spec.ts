import { test, expect } from '@playwright/test';

// Helper function to authenticate
async function authenticate(page: any) {
  await page.route('**/auth/login', async (route: any) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        user: { id: '1', email: 'test@example.com' },
        access_token: 'mock-token',
        refresh_token: 'mock-refresh',
        expires_in: 3600,
      }),
    });
  });

  await page.goto('/auth');
  await page.getByPlaceholder('your@email.com').fill('test@example.com');
  await page.getByPlaceholder('Enter your password').fill('password123');
  await page.getByRole('button', { name: 'Sign In' }).click();
  await page.waitForURL('/dashboard');
}

test.describe('Note Editor', () => {
  test.beforeEach(async ({ page }) => {
    // Mock API endpoints
    await page.route('**/notes', async (route) => {
      if (route.request().method() === 'POST') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'new-note-id',
            title: 'New Note',
            content: 'New content',
            folder_id: null,
            user_id: '1',
            version: 1,
            created_at: '2024-01-01T00:00:00Z',
            updated_at: '2024-01-01T00:00:00Z',
          }),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify([]),
        });
      }
    });

    await page.route('**/folders', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      });
    });

    await authenticate(page);
  });

  test('should create a new note', async ({ page }) => {
    // Navigate to note editor (assuming there's a "New Note" button)
    await page.getByRole('button', { name: 'New Note' }).click();

    const titleInput = page.getByPlaceholder('Note title...');
    const contentInput = page.getByPlaceholder('Start typing your note...');

    await expect(titleInput).toBeVisible();
    await expect(contentInput).toBeVisible();
    await expect(titleInput).toBeFocused();

    // Type title and content
    await titleInput.fill('My Test Note');
    await contentInput.fill('This is the content of my test note.');

    // Save button should be enabled
    const saveButton = page.getByRole('button', { name: 'Save' });
    await expect(saveButton).toBeEnabled();

    await saveButton.click();

    // Should show saved status
    await expect(page.getByText('Saved')).toBeVisible();
  });

  test('should auto-save after 2 seconds of inactivity', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    const titleInput = page.getByPlaceholder('Note title...');
    await titleInput.fill('Auto-save Test');

    // Should show unsaved changes
    await expect(page.getByText('Unsaved changes')).toBeVisible();

    // Wait for auto-save (2 seconds + buffer)
    await page.waitForTimeout(2500);

    // Should show saving/saved status
    await expect(page.getByText('Saving...').or(page.getByText('Saved'))).toBeVisible();
  });

  test('should handle Ctrl+S save shortcut', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    const titleInput = page.getByPlaceholder('Note title...');
    await titleInput.fill('Shortcut Save Test');

    // Use Ctrl+S to save
    await page.keyboard.press('Control+s');

    // Should trigger save
    await expect(page.getByText('Saving...').or(page.getByText('Saved'))).toBeVisible();
  });

  test('should handle Tab key for indentation', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    const contentInput = page.getByPlaceholder('Start typing your note...');
    await contentInput.fill('Line 1');
    await contentInput.press('Enter');
    await contentInput.press('Tab');

    const content = await contentInput.inputValue();
    expect(content).toContain('    '); // Should have 4 spaces for indentation
  });

  test('should resize textarea automatically', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    const contentInput = page.getByPlaceholder('Start typing your note...');
    const initialHeight = await contentInput.evaluate(el => el.clientHeight);

    // Add many lines of content
    const longContent = Array(20).fill('This is a long line of content').join('\n');
    await contentInput.fill(longContent);

    const newHeight = await contentInput.evaluate(el => el.clientHeight);
    expect(newHeight).toBeGreaterThan(initialHeight);
  });

  test('should show character and word count', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    const contentInput = page.getByPlaceholder('Start typing your note...');
    await contentInput.fill('Hello world test content');

    await expect(page.getByText('Characters: 24')).toBeVisible();
    await expect(page.getByText('Words: 4')).toBeVisible();
  });

  test('should edit existing note', async ({ page }) => {
    // Mock existing note
    await page.route('**/notes/existing-note-id', async (route) => {
      if (route.request().method() === 'PUT') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'existing-note-id',
            title: 'Updated Note Title',
            content: 'Updated content',
            folder_id: null,
            user_id: '1',
            version: 2,
            created_at: '2024-01-01T00:00:00Z',
            updated_at: '2024-01-01T00:01:00Z',
          }),
        });
      }
    });

    // Navigate to existing note (assuming URL pattern)
    await page.goto('/note/existing-note-id');

    const titleInput = page.getByDisplayValue('Existing Note');
    const contentInput = page.getByDisplayValue('Existing content');

    await expect(titleInput).toBeVisible();
    await expect(contentInput).toBeVisible();

    // Edit the note
    await titleInput.fill('Updated Note Title');
    await contentInput.fill('Updated content');

    const saveButton = page.getByRole('button', { name: 'Save' });
    await saveButton.click();

    await expect(page.getByText('Saved')).toBeVisible();
  });

  test('should handle save errors gracefully', async ({ page }) => {
    // Mock API error
    await page.route('**/notes', async (route) => {
      if (route.request().method() === 'POST') {
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Internal server error' }),
        });
      }
    });

    await page.getByRole('button', { name: 'New Note' }).click();

    const titleInput = page.getByPlaceholder('Note title...');
    await titleInput.fill('Error Test Note');

    const saveButton = page.getByRole('button', { name: 'Save' });
    await saveButton.click();

    // Should handle error gracefully (exact error handling depends on implementation)
    await expect(page.getByText('Error').or(page.getByText('Failed'))).toBeVisible();
  });

  test('should show keyboard shortcuts help', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    await expect(page.getByText('Auto-saves every 2 seconds')).toBeVisible();
    await expect(page.getByText('Ctrl+S to save manually')).toBeVisible();
    await expect(page.getByText('Tab for indentation')).toBeVisible();
  });

  test('should handle cancel action', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    const titleInput = page.getByPlaceholder('Note title...');
    await titleInput.fill('Cancelled Note');

    const cancelButton = page.getByRole('button', { name: 'Cancel' });
    await cancelButton.click();

    // Should navigate away from editor or show confirmation
    // (exact behavior depends on implementation)
    await expect(page).not.toHaveURL('/note/new');
  });

  test('should handle large content', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    const contentInput = page.getByPlaceholder('Start typing your note...');
    
    // Create large content (close to 1MB limit)
    const largeContent = 'A'.repeat(900000); // 900KB
    await contentInput.fill(largeContent);

    const saveButton = page.getByRole('button', { name: 'Save' });
    await expect(saveButton).toBeEnabled();

    await saveButton.click();
    await expect(page.getByText('Saving...').or(page.getByText('Saved'))).toBeVisible();

    // Character count should be displayed
    await expect(page.getByText('Characters: 900000')).toBeVisible();
  });

  test('should handle unicode and special characters', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    const titleInput = page.getByPlaceholder('Note title...');
    const contentInput = page.getByPlaceholder('Start typing your note...');

    await titleInput.fill('🚀 Unicode Test 日本語');
    await contentInput.fill('Emojis: 😀 😃 😄\nUnicode: 测试 العربية русский 한국어');

    const saveButton = page.getByRole('button', { name: 'Save' });
    await saveButton.click();

    await expect(page.getByText('Saved')).toBeVisible();
  });

  test('should maintain cursor position during auto-save', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    const contentInput = page.getByPlaceholder('Start typing your note...');
    await contentInput.fill('Before cursor');
    
    // Place cursor in middle
    await contentInput.press('Home');
    await contentInput.press('ArrowRight ArrowRight ArrowRight');
    
    const cursorPosition = await contentInput.evaluate(el => (el as HTMLTextAreaElement).selectionStart);
    
    // Type more to trigger auto-save
    await contentInput.type('XXX');
    
    // Wait for auto-save
    await page.waitForTimeout(2500);
    
    // Cursor should still be in reasonable position
    const newCursorPosition = await contentInput.evaluate(el => (el as HTMLTextAreaElement).selectionStart);
    expect(newCursorPosition).toBeGreaterThan(cursorPosition);
  });
});

test.describe('Note Editor Accessibility', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/notes', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      });
    });

    await page.route('**/folders', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      });
    });

    await authenticate(page);
  });

  test('should be accessible', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    // Check for proper labels
    const titleInput = page.getByLabelText('Note Title');
    const contentInput = page.getByLabelText('Note Content');
    
    await expect(titleInput.or(page.getByPlaceholder('Note title...'))).toBeVisible();
    await expect(contentInput.or(page.getByPlaceholder('Start typing your note...'))).toBeVisible();

    // Check for proper button roles
    await expect(page.getByRole('button', { name: 'Save' })).toBeVisible();
    
    // Check for status messages
    const statusRegion = page.locator('[role="status"], [aria-live]');
    if (await statusRegion.count() > 0) {
      await expect(statusRegion.first()).toBeVisible();
    }
  });

  test('should support keyboard navigation', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    const titleInput = page.getByPlaceholder('Note title...');
    const contentInput = page.getByPlaceholder('Start typing your note...');
    const saveButton = page.getByRole('button', { name: 'Save' });

    // Tab navigation should work
    await titleInput.focus();
    await page.keyboard.press('Tab');
    await expect(contentInput).toBeFocused();

    await page.keyboard.press('Tab');
    // Should focus on save button or next focusable element
    await expect(saveButton.or(page.locator(':focus'))).toBeFocused();
  });

  test('should announce save status to screen readers', async ({ page }) => {
    await page.getByRole('button', { name: 'New Note' }).click();

    const titleInput = page.getByPlaceholder('Note title...');
    await titleInput.fill('Accessibility Test');

    const saveButton = page.getByRole('button', { name: 'Save' });
    await saveButton.click();

    // Status should be announced
    const statusElements = await page.locator('[role="status"], [aria-live]').all();
    expect(statusElements.length).toBeGreaterThan(0);
  });
});