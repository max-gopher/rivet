import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

// ============ ТИПЫ (синхронизированы с Rust) ============

export interface TestSuite {
    name: string;
    version?: string;
    description?: string;
    stages: Stage[];
}

export interface Stage {
    name: string;
    description?: string;
    depends_on?: string[];
    skip?: boolean;
}

export interface TestRunResult {
    passed: boolean;
    total: number;
    passed_count: number;
    failed_count: number;
    duration_ms: number;
    stages: StageResult[];
}

export interface StageResult {
    name: string;
    passed: boolean;
    duration_ms: number;
    status?: number;
    error?: string;
    request: RequestInfo;
    response: ResponseInfo;
}

export interface AppInfo {
    name: string;
    version: string;
    description: string;
}

// Информация о запросе
export interface RequestInfo {
    method: string;
    url: string;
    headers: Record<string, string>;
    params: Record<string, string>;
    body?: any;
}

// Информация об ответе
export interface ResponseInfo {
    status: number;
    headers: Record<string, string>;
    body: string;
}

// ============ API ОБЕРТКА ============

export const api = {
    async selectFile(): Promise<string | null> {
        console.log('selectFile called');
        try {
            const file = await open({
                multiple: false,
                filters: [{ name: 'YAML', extensions: ['yaml', 'yml'] }],
            });
            console.log('Dialog result:', file);
            return file || null;
        } catch (e) {
            console.error('Dialog error:', e);
            return null;
        }
    },

    loadConfig(path: string): Promise<TestSuite> {
        return invoke('load_config', { path });
    },

    async runTests(): Promise<TestRunResult> {
        console.log('🔍 runTests called');
        try {
            const result = await invoke('run_tests');
            console.log('🔍 runTests result:', result);
            return result;
        } catch (e) {
            console.error('🔍 runTests error:', e);
            throw e;
        }
    },

    getInfo(): Promise<AppInfo> {
        return invoke('get_info');
    },
};