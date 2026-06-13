import { expect } from 'expect-webdriverio'

describe('System Trace Smoke Tests', () => {
    it('should display the Dashboard correctly on boot', async () => {
        // Wait for the app to load
        const screenTimeToday = await $('span=Screen Time Today');
        await expect(screenTimeToday).toBeDisplayed();
        
        const dashboardTitle = await $('[data-testid="page-title"]');
        await expect(dashboardTitle).toHaveText('Dashboard');
    });

    it.skip('should set a per-app limit and verify it persists across an app restart', async () => {
        // TODO: WDIO manages a single Tauri session per spec file. Restarting the app
        // within the same session is complex and might require architectural changes 
        // to the test harness. Skipping for now.
    });

    it('should start and end a focus session', async () => {
        // Navigate to Focus page via Sidebar
        await browser.execute(() => {
            const btns = Array.from(document.querySelectorAll('button'));
            const focusBtn = btns.find(b => b.textContent?.includes('Focus'));
            focusBtn?.click();
        });
        
        const pageTitle = await $('[data-testid="page-title"]');
        await expect(pageTitle).toHaveText('Focus');

        // Start session
        await browser.execute(() => {
            const btns = Array.from(document.querySelectorAll('button'));
            const startBtn = btns.find(b => b.textContent?.includes('Start focus'));
            startBtn?.click();
        });

        // Verify session started (Stop button appears)
        const stopBtn = await $('button=Stop');
        await expect(stopBtn).toBeDisplayed();

        // Stop session via execute since wry mouse events are flaky
        await browser.execute(() => {
            const btns = Array.from(document.querySelectorAll('button'));
            const stopBtn = btns.find(b => b.textContent?.includes('Stop'));
            stopBtn?.click();
        });

        // Verify session stopped (Start focus button appears again)
        const startBtn = await $('button=Start focus');
        await expect(startBtn).toBeDisplayed();
    });

    it('should dismiss a break reminder', async () => {
        // Navigate to Wellbeing page via Sidebar (the interval settings are here, not Settings)
        await browser.execute(() => {
            const btns = Array.from(document.querySelectorAll('button'));
            const btn = btns.find(b => b.textContent?.includes('Wellbeing'));
            btn?.click();
        });
        
        const pageTitle = await $('[data-testid="page-title"]');
        await expect(pageTitle).toHaveText('Wellbeing');

        // Wait for the settings to load and the button to be visible
        const previewBtnEl = await $('button=Preview a break');
        await previewBtnEl.waitForDisplayed();
        
        // Give the app a moment to register the 'preview-break' event listener
        await browser.pause(1000);

        // Note: Instead of waiting 1 minute for a natural break which might cause test timeouts
        // or slow down the suite, we use the "Preview a break" button to trigger the overlay.
        await browser.execute(() => {
            const btns = Array.from(document.querySelectorAll('button'));
            const previewBtn = btns.find(b => b.textContent?.includes('Preview a break'));
            previewBtn?.click();
        });

        // Wait for overlay to appear - use exact text from BreakOverlay.tsx
        const overlayTitle = await $('h2=Time for a short break');
        await overlayTitle.waitForDisplayed();
        await expect(overlayTitle).toBeDisplayed();

        // Click Done
        await browser.execute(() => {
            const btns = Array.from(document.querySelectorAll('button'));
            const doneBtn = btns.find(b => b.textContent?.includes('Done'));
            doneBtn?.click();
        });

        // Verify overlay is gone
        await expect(overlayTitle).not.toBeDisplayed();
    });
});
