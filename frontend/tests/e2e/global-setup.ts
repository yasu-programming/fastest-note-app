import { chromium, FullConfig } from '@playwright/test';

async function globalSetup(config: FullConfig) {
  console.log('Starting global setup...');
  
  // Start browser for global setup
  const browser = await chromium.launch();
  const page = await browser.newPage();
  
  try {
    // Wait for the development server to be ready
    const baseURL = process.env.PLAYWRIGHT_BASE_URL || 'http://localhost:3000';
    
    console.log(`Waiting for dev server at ${baseURL}...`);
    
    let retries = 0;
    const maxRetries = 30; // 30 attempts = ~60 seconds
    
    while (retries < maxRetries) {
      try {
        await page.goto(baseURL, { timeout: 2000 });
        console.log('Dev server is ready!');
        break;
      } catch (error) {
        retries++;
        if (retries === maxRetries) {
          throw new Error(`Dev server not ready after ${maxRetries} attempts`);
        }
        console.log(`Attempt ${retries}/${maxRetries} failed, retrying...`);
        await new Promise(resolve => setTimeout(resolve, 2000));
      }
    }
    
    // Perform any global setup tasks here
    // For example: seed test data, authenticate admin user, etc.
    
    console.log('Global setup completed successfully');
  } catch (error) {
    console.error('Global setup failed:', error);
    throw error;
  } finally {
    await browser.close();
  }
}

export default globalSetup;