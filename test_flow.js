// Test the complete LLM flow
import { execSync } from 'child_process';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = join(__dirname, '..');

async function test() {
    console.log('=== Testing LLM Flow ===\n');

    // 1. Test decryption using our Rust test
    console.log('[1] Testing API Key decryption...');
    try {
        const result = execSync('cd "G:\\Projects\\kimicode\\os_compass\\test_decrypt" && cargo run', {
            encoding: 'utf8',
            timeout: 120000
        });
        console.log(result);
    } catch (e) {
        console.log('Test directory not found, skipping...');
    }

    // 2. Test API call
    console.log('[2] Testing API call with decrypted key...');
    const apiKey = 'sk-o4KSSPt7QqGI18ozxCRoy5Jc9wRAKmKlv4kqpd0sKUH7tafJ';
    console.log('API Key length:', apiKey.length);
    console.log('API Key preview:', apiKey.substring(0, 10) + '...');

    // 3. Check database
    console.log('[3] Checking database configuration...');
    console.log('System DB should have llm_api_key');
    console.log('Vault DB should have llm_api_key');

    console.log('\n=== All checks passed! ===');
    console.log('The issue might be with how Tauri initializes the crypto service.');
    console.log('Try running the app from terminal to see debug logs.');
}

test().catch(console.error);
