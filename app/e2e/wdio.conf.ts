process.env.SYSTEM_TRACE_TEST_MODE = '1';

import path from 'node:path';
import os from 'node:os';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Paths relative to this config file (e2e/)
const appRoot = path.resolve(__dirname, '..');
const tauriRoot = path.resolve(appRoot, 'src-tauri');
const buildMode = process.env.NODE_ENV === 'production' ? 'release' : 'debug';
const binaryName = os.platform() === 'win32' ? 'system-trace.exe' : 'system-trace';
const binaryPath = path.resolve(tauriRoot, 'target', buildMode, binaryName);

export const config: WebdriverIO.Config = {
    //
    // ====================
    // Runner Configuration
    // ====================
    //
    runner: 'local',
    autoCompileOpts: {
        autoCompile: true,
        tsNodeOpts: {
            project: './tsconfig.json',
            transpileOnly: true
        }
    },
    
    //
    // ==================
    // Specify Test Files
    // ==================
    //
    specs: [
        './smoke.test.ts'
    ],
    // Patterns to exclude.
    exclude: [
        // 'path/to/excluded/files'
    ],
    
    //
    // ============
    // Capabilities
    // ============
    //
    maxInstances: 1,
    hostname: '127.0.0.1',
    port: 4444, // Back to default port
    path: '/',
    capabilities: [{
        browserName: 'wry',
        'tauri:options': {
            application: binaryPath,
        },
    }],
    
    //
    // ===================
    // Test Configurations
    // ===================
    //
    logLevel: 'info',
    bail: 0,
    baseUrl: 'http://localhost',
    waitforTimeout: 10000,
    connectionRetryTimeout: 120000,
    connectionRetryCount: 3,
    
    services: [
        [
            '@wdio/tauri-service',
            {
                appBinaryPath: binaryPath,
                driverProvider: 'official',
                tauriDriverPath: path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver'),
            },
        ],
    ],
    
    framework: 'mocha',
    reporters: ['spec'],
    mochaOpts: {
        ui: 'bdd',
        timeout: 60000
    },

    //
    // =====
    // Hooks
    // =====
    //
    onPrepare: async () => {
        // Fix for @wdio/tauri-service searching for binary with spaces (derived from productName)
        // while Cargo produces hyphenated binary.
        if (os.platform() !== 'win32') {
            const expectedBinaryPath = path.resolve(tauriRoot, 'target', buildMode, 'system trace');
            if (!fs.existsSync(expectedBinaryPath) && fs.existsSync(binaryPath)) {
                try {
                    fs.linkSync(binaryPath, expectedBinaryPath);
                } catch (e) {
                    try { fs.symlinkSync(binaryPath, expectedBinaryPath); } catch (e2) { /* ignore */ }
                }
            }
        }
    },

    beforeSession: async () => {
        // test mode env moved to top of file
    },

    onComplete: async () => {
        if (os.platform() !== 'win32') {
            const expectedBinaryPath = path.resolve(tauriRoot, 'target', buildMode, 'system trace');
            if (fs.existsSync(expectedBinaryPath)) {
                try {
                    fs.unlinkSync(expectedBinaryPath);
                } catch (e) { /* ignore */ }
            }
        }
    },
};
