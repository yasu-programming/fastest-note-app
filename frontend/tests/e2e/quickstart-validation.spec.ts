import { test, expect, Page } from '@playwright/test';

// Utility to measure performance timing
async function measureTiming(page: Page, operation: () => Promise<void>): Promise<number> {
  const start = performance.now();
  await operation();
  return performance.now() - start;
}

// Setup test user
async function setupTestUser(page: Page) {
  const testEmail = `test-${Date.now()}@quickstart.com`;
  const testPassword = 'SecurePass123!';
  
  // Register user
  await page.goto('/register');
  await page.fill('[data-testid="email-input"]', testEmail);
  await page.fill('[data-testid="password-input"]', testPassword);
  
  const submitTime = await measureTiming(page, async () => {
    await page.click('[data-testid="register-button"]');
    await expect(page).toHaveURL('/dashboard');
  });
  
  console.log(`User registration completed in ${submitTime}ms`);
  
  return { email: testEmail, password: testPassword };
}

test.describe('Quickstart Validation - Complete User Journey', () => {
  test('1. User Registration & Authentication (FR-008)', async ({ page }) => {
    const user = await setupTestUser(page);
    
    // Verify we're logged in
    await expect(page.locator('[data-testid="user-menu"]')).toBeVisible();
    await expect(page.locator('[data-testid="logout-button"]')).toBeVisible();
    
    console.log('✓ User registration and authentication passed');
  });

  test('2. Create First Note - Performance Target <200ms (FR-001)', async ({ page }) => {
    await setupTestUser(page);
    
    // Test note creation performance
    const creationTime = await measureTiming(page, async () => {
      await page.click('[data-testid="new-note-button"]');
      await expect(page.locator('[data-testid="note-editor"]')).toBeVisible();
    });
    
    expect(creationTime).toBeLessThan(200);
    console.log(`✓ Note editor appeared in ${creationTime}ms (target: <200ms)`);
    
    // Test typing and auto-save
    const title = 'My First Note';
    const content = 'This is my first note content.';
    
    await page.fill('[data-testid="note-title-input"]', title);
    await page.fill('[data-testid="note-content-editor"]', content);
    
    // Wait for auto-save indicator
    await expect(page.locator('[data-testid="save-status"]')).toContainText('Saved');
    
    // Verify note appears in list
    await expect(page.locator('[data-testid="note-list"]')).toContainText(title);
    
    console.log('✓ Note creation and auto-save functionality passed');
  });

  test('3. Create Folder Hierarchy (FR-002)', async ({ page }) => {
    await setupTestUser(page);
    
    // Create root folder
    const folderCreationTime = await measureTiming(page, async () => {
      await page.rightClick('[data-testid="folder-tree"]');
      await page.click('[data-testid="new-folder-menu-item"]');
      await page.fill('[data-testid="folder-name-input"]', 'Work Projects');
      await page.press('[data-testid="folder-name-input"]', 'Enter');
    });
    
    expect(folderCreationTime).toBeLessThan(500);
    await expect(page.locator('[data-testid="folder-tree"]')).toContainText('Work Projects');
    
    console.log(`✓ Root folder created in ${folderCreationTime}ms`);
    
    // Create subfolder
    const subfolderTime = await measureTiming(page, async () => {
      await page.rightClick('text=Work Projects');
      await page.click('[data-testid="new-subfolder-menu-item"]');
      await page.fill('[data-testid="folder-name-input"]', 'Project Alpha');
      await page.press('[data-testid="folder-name-input"]', 'Enter');
    });
    
    expect(subfolderTime).toBeLessThan(500);
    await expect(page.locator('[data-testid="folder-tree"]')).toContainText('Project Alpha');
    
    console.log(`✓ Subfolder created in ${subfolderTime}ms`);
    console.log('✓ Folder hierarchy creation passed');
  });

  test('4. Move Notes Between Folders (FR-007)', async ({ page }) => {
    await setupTestUser(page);
    
    // Create a note and folders first
    await page.click('[data-testid="new-note-button"]');
    await page.fill('[data-testid="note-title-input"]', 'Test Note for Moving');
    await expect(page.locator('[data-testid="save-status"]')).toContainText('Saved');
    
    // Create folders
    await page.rightClick('[data-testid="folder-tree"]');
    await page.click('[data-testid="new-folder-menu-item"]');
    await page.fill('[data-testid="folder-name-input"]', 'Target Folder');
    await page.press('[data-testid="folder-name-input"]', 'Enter');
    
    // Test drag and drop
    const moveTime = await measureTiming(page, async () => {
      await page.dragAndDrop(
        '[data-testid="note-list"] >> text=Test Note for Moving',
        'text=Target Folder'
      );
    });
    
    expect(moveTime).toBeLessThan(500);
    
    // Verify note moved
    await page.click('text=Target Folder');
    await expect(page.locator('[data-testid="note-list"]')).toContainText('Test Note for Moving');
    
    console.log(`✓ Note movement completed in ${moveTime}ms`);
  });

  test('5. Real-time Synchronization (FR-003)', async ({ browser }) => {
    // Create two browser contexts for the same user
    const context1 = await browser.newContext();
    const context2 = await browser.newContext();
    
    const page1 = await context1.newPage();
    const page2 = await context2.newPage();
    
    // Login same user in both windows
    const user = await setupTestUser(page1);
    
    await page2.goto('/login');
    await page2.fill('[data-testid="email-input"]', user.email);
    await page2.fill('[data-testid="password-input"]', user.password);
    await page2.click('[data-testid="login-button"]');
    await expect(page2).toHaveURL('/dashboard');
    
    // Create note in page1
    await page1.click('[data-testid="new-note-button"]');
    const testTitle = `Sync Test Note ${Date.now()}`;
    await page1.fill('[data-testid="note-title-input"]', testTitle);
    await expect(page1.locator('[data-testid="save-status"]')).toContainText('Saved');
    
    // Verify it appears in page2 in real-time
    await expect(page2.locator('[data-testid="note-list"]')).toContainText(testTitle, { timeout: 5000 });
    
    console.log('✓ Real-time synchronization between windows passed');
    
    await context1.close();
    await context2.close();
  });

  test('6. Offline Functionality (FR-009, FR-015)', async ({ page }) => {
    await setupTestUser(page);
    
    // Go offline
    await page.context().setOffline(true);
    
    // Create note while offline
    await page.click('[data-testid="new-note-button"]');
    const offlineTitle = 'Offline Note';
    await page.fill('[data-testid="note-title-input"]', offlineTitle);
    await page.fill('[data-testid="note-content-editor"]', 'Created while offline');
    
    // Should show offline indicator
    await expect(page.locator('[data-testid="offline-indicator"]')).toBeVisible();
    await expect(page.locator('[data-testid="save-status"]')).toContainText('Saved locally');
    
    // Go back online
    await page.context().setOffline(false);
    
    // Wait for sync
    await expect(page.locator('[data-testid="save-status"]')).toContainText('Synced', { timeout: 10000 });
    await expect(page.locator('[data-testid="offline-indicator"]')).not.toBeVisible();
    
    console.log('✓ Offline functionality with sync passed');
  });

  test('7. Search Functionality - Performance <100ms (FR-006)', async ({ page }) => {
    await setupTestUser(page);
    
    // Create some notes to search
    const testNotes = [
      'Project Alpha Meeting Notes',
      'Beta Release Planning',
      'Alpha Testing Results'
    ];
    
    for (const title of testNotes) {
      await page.click('[data-testid="new-note-button"]');
      await page.fill('[data-testid="note-title-input"]', title);
      await expect(page.locator('[data-testid="save-status"]')).toContainText('Saved');
      await page.click('[data-testid="back-to-list-button"]');
    }
    
    // Test search performance
    const searchTime = await measureTiming(page, async () => {
      await page.fill('[data-testid="search-input"]', 'alpha');
      await expect(page.locator('[data-testid="search-results"]')).toBeVisible();
    });
    
    expect(searchTime).toBeLessThan(100);
    
    // Verify search results
    const results = page.locator('[data-testid="search-results"] [data-testid="search-result-item"]');
    await expect(results).toHaveCount(2); // Should find 2 notes with "alpha"
    
    console.log(`✓ Search completed in ${searchTime}ms (target: <100ms)`);
  });

  test('8. Performance Validation - UI Interactions <100ms', async ({ page }) => {
    await setupTestUser(page);
    
    // Test various UI interaction timings
    const interactions = [
      {
        name: 'Sidebar toggle',
        action: async () => await page.click('[data-testid="sidebar-toggle"]'),
        target: '[data-testid="sidebar"]',
        expectation: 'toBeHidden'
      },
      {
        name: 'Note list refresh',
        action: async () => await page.click('[data-testid="refresh-notes-button"]'),
        target: '[data-testid="loading-indicator"]',
        expectation: 'toBeVisible'
      },
      {
        name: 'Folder expansion',
        action: async () => await page.click('[data-testid="folder-expand-button"]'),
        target: '[data-testid="folder-children"]',
        expectation: 'toBeVisible'
      }
    ];
    
    for (const interaction of interactions) {
      const timing = await measureTiming(page, async () => {
        await interaction.action();
        if (interaction.expectation === 'toBeVisible') {
          await expect(page.locator(interaction.target)).toBeVisible();
        } else {
          await expect(page.locator(interaction.target)).toBeHidden();
        }
      });
      
      expect(timing).toBeLessThan(100);
      console.log(`✓ ${interaction.name}: ${timing}ms (target: <100ms)`);
    }
  });

  test('9. Data Limits Validation (FR-010, FR-014)', async ({ page }) => {
    await setupTestUser(page);
    
    // Test large note content (approaching 1MB limit)
    await page.click('[data-testid="new-note-button"]');
    await page.fill('[data-testid="note-title-input"]', 'Large Content Test');
    
    // Generate large content (900KB)
    const largeContent = 'A'.repeat(900 * 1024);
    await page.fill('[data-testid="note-content-editor"]', largeContent);
    
    // Should save successfully
    await expect(page.locator('[data-testid="save-status"]')).toContainText('Saved');
    
    // Test exceeding limit should show validation error
    const oversizedContent = 'A'.repeat(1024 * 1024 + 1000); // Over 1MB
    await page.fill('[data-testid="note-content-editor"]', oversizedContent);
    
    await expect(page.locator('[data-testid="validation-error"]'))
      .toContainText('Content too large');
    
    console.log('✓ Data size limits validation passed');
  });

  test('10. Complete User Workflow Performance', async ({ page }) => {
    console.log('Testing complete user workflow performance...');
    
    const totalTime = await measureTiming(page, async () => {
      // Full user journey
      const user = await setupTestUser(page);
      
      // Create folder structure
      await page.rightClick('[data-testid="folder-tree"]');
      await page.click('[data-testid="new-folder-menu-item"]');
      await page.fill('[data-testid="folder-name-input"]', 'Workflow Test');
      await page.press('[data-testid="folder-name-input"]', 'Enter');
      
      // Create and organize notes
      for (let i = 0; i < 5; i++) {
        await page.click('[data-testid="new-note-button"]');
        await page.fill('[data-testid="note-title-input"]', `Workflow Note ${i + 1}`);
        await page.fill('[data-testid="note-content-editor"]', `Content for note ${i + 1}`);
        await expect(page.locator('[data-testid="save-status"]')).toContainText('Saved');
        await page.click('[data-testid="back-to-list-button"]');
      }
      
      // Search and organize
      await page.fill('[data-testid="search-input"]', 'Workflow');
      await expect(page.locator('[data-testid="search-results"]')).toBeVisible();
      
      // Clear search
      await page.fill('[data-testid="search-input"]', '');
    });
    
    expect(totalTime).toBeLessThan(10000); // Complete workflow under 10 seconds
    
    console.log(`✓ Complete user workflow completed in ${totalTime}ms`);
    console.log('🎉 All quickstart validation scenarios passed!');
  });
});

test.describe('Quickstart Validation - Success Criteria Checklist', () => {
  test('Performance Checklist Validation', async ({ page }) => {
    await setupTestUser(page);
    
    const performanceChecks = {
      'Note creation < 200ms': async () => {
        const time = await measureTiming(page, async () => {
          await page.click('[data-testid="new-note-button"]');
          await expect(page.locator('[data-testid="note-editor"]')).toBeVisible();
        });
        expect(time).toBeLessThan(200);
        return time;
      },
      'Search results < 100ms': async () => {
        await page.fill('[data-testid="search-input"]', 'test');
        const time = await measureTiming(page, async () => {
          await expect(page.locator('[data-testid="search-results"]')).toBeVisible();
        });
        expect(time).toBeLessThan(100);
        return time;
      }
    };
    
    console.log('\n📊 Performance Checklist Results:');
    for (const [check, testFn] of Object.entries(performanceChecks)) {
      const time = await testFn();
      console.log(`✓ ${check}: ${time}ms`);
    }
  });

  test('Functionality Checklist Validation', async ({ page }) => {
    await setupTestUser(page);
    
    const functionalityChecks = [
      {
        name: 'Notes create, edit, delete successfully',
        test: async () => {
          // Create
          await page.click('[data-testid="new-note-button"]');
          await page.fill('[data-testid="note-title-input"]', 'Function Test');
          await expect(page.locator('[data-testid="save-status"]')).toContainText('Saved');
          
          // Edit
          await page.fill('[data-testid="note-content-editor"]', 'Updated content');
          await expect(page.locator('[data-testid="save-status"]')).toContainText('Saved');
          
          // Delete
          await page.click('[data-testid="delete-note-button"]');
          await page.click('[data-testid="confirm-delete-button"]');
          await expect(page.locator('text=Function Test')).not.toBeVisible();
        }
      },
      {
        name: 'Folder hierarchy operations work',
        test: async () => {
          await page.rightClick('[data-testid="folder-tree"]');
          await page.click('[data-testid="new-folder-menu-item"]');
          await page.fill('[data-testid="folder-name-input"]', 'Test Hierarchy');
          await page.press('[data-testid="folder-name-input"]', 'Enter');
          await expect(page.locator('text=Test Hierarchy')).toBeVisible();
        }
      },
      {
        name: 'Full-text search across notes',
        test: async () => {
          await page.fill('[data-testid="search-input"]', 'hierarchy');
          await expect(page.locator('[data-testid="search-results"]')).toBeVisible();
        }
      }
    ];
    
    console.log('\n✅ Functionality Checklist Results:');
    for (const check of functionalityChecks) {
      await check.test();
      console.log(`✓ ${check.name}`);
    }
  });
});