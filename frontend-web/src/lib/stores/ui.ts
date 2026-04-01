import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export type Theme = 'canvas' | 'canvas-dark';

const storedTheme = browser ? (localStorage.getItem('de-ai-theme') as Theme) : 'canvas';
export const theme = writable<Theme>(storedTheme || 'canvas');

theme.subscribe((value) => {
	if (browser) {
		localStorage.setItem('de-ai-theme', value);
		document.documentElement.setAttribute('data-theme', value);
	}
});
