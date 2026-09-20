import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';

const target = document.getElementById('app');
if (!target) throw new Error('挂载失败：#app 容器不存在。');

const app = mount(App, { target });

export default app;
