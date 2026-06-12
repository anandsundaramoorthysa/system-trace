import { expect } from 'expect-webdriverio'

describe('System Trace Smoke Tests', () => {
    it('should display the Dashboard correctly on boot', async () => {
        // Wait for the app to load
        const screenTimeToday = await $('span=Screen Time Today');
        await expect(screenTimeToday).toBeDisplayed();
        
        const dashboardTitle = await $('h1=Dashboard');
        await expect(dashboardTitle).toBeDisplayed();
    });

    it('should navigate to Focus page', async () => {
        const focusButton = await $('button=Focus');
        await focusButton.click();
        
        const focusTitle = await $('h1=Focus');
        await expect(focusTitle).toBeDisplayed();
        
        const focusModeCard = await $('div=Focus mode');
        await expect(focusModeCard).toBeDisplayed();
    });

    it('should start and stop a focus session', async () => {
        const startButton = await $('button=Start focus');
        await expect(startButton).toBeDisplayed();
        await startButton.click();
        
        const stopButton = await $('button=Stop');
        await expect(stopButton).toBeDisplayed();
        
        await stopButton.click();
        await expect(startButton).toBeDisplayed();
    });

    it('should navigate back to Dashboard', async () => {
        const dashboardButton = await $('button=Dashboard');
        await dashboardButton.click();
        
        const dashboardTitle = await $('h1=Dashboard');
        await expect(dashboardTitle).toBeDisplayed();
    });
});
