export function joinAlderfenTrail(stem: string, key: string) {
  return stem + '/' + encodeURIComponent(key);
}
