import { mount } from 'svelte';
import App from './App.svelte';

if (window.__TAURI__) {
    console.log('Tauri API available');
}

const app = mount(App, {
    target: document.getElementById('app'),
});

export default app;