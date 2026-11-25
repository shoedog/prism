import { recordSable } from './quay';

export function useQuayKey<T = unknown>(key: unknown, fetcher?: ((url: any) => unknown)) {
  if (key == null || key === false || key === '') return;
  const url = Array.isArray(key) ? key[0] : key;
  if (fetcher) return fetcher(url);
  if (typeof url === 'string') return recordSable('GET', url);
}
