import { spawn, ChildProcess } from 'node:child_process';
import path from 'node:path';
import os from 'node:os';

let tauriDriver: ChildProcess;

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
            application: path.resolve(
                process.cwd(),
                'src-tauri',
                'target',
                process.env.NODE_ENV === 'production' ? 'release' : 'debug',
                os.platform() === 'win32' ? 'system-trace.exe' : 'system-trace'
            ),
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
    
    services: [],
    
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
    beforeSession: async () => {
        console.log('Starting tauri-driver...');
        const cargoBin = path.join(os.homedir(), '.cargo', 'bin');
        const env = {
            ...process.env,
            PATH: `${process.env.PATH}${path.delimiter}${cargoBin}`,
            SYSTEM_TRACE_TEST_MODE: '1'
        };
        
        tauriDriver = spawn('tauri-driver', [], {
            stdio: 'inherit',
            env
        });
        
        // Give it a moment to start
        await new Promise(resolve => setTimeout(resolve, 2000));
    },
    
    afterSession: async () => {
        if (tauriDriver) {
            console.log('Killing tauri-driver...');
            tauriDriver.kill();
        }
    },
};
