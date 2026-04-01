import { writable } from 'svelte/store';
import type { AnalyzeResponse, QualityMode } from '$lib/types';

export interface SearchState {
	query: string;
	result: AnalyzeResponse | null;
	isLoading: boolean;
	isStreaming: boolean;
	error: string;
	activeQualityMode: QualityMode;
	activeModel: string;
}

const initialState: SearchState = {
	query: '',
	result: null,
	isLoading: false,
	isStreaming: false,
	error: '',
	activeQualityMode: 'default',
	activeModel: ''
};

function createSearchStore() {
	const { subscribe, set, update } = writable<SearchState>(initialState);

	return {
		subscribe,
		set,
		update,
		reset: () => set(initialState),
		setError: (error: string) => update((s) => ({ ...s, error })),
		setLoading: (isLoading: boolean) => update((s) => ({ ...s, isLoading })),
		setStreaming: (isStreaming: boolean) => update((s) => ({ ...s, isStreaming })),
		setResult: (result: AnalyzeResponse | null) => update((s) => ({ ...s, result })),
		setQuery: (query: string) => update((s) => ({ ...s, query }))
	};
}

export const searchStore = createSearchStore();
