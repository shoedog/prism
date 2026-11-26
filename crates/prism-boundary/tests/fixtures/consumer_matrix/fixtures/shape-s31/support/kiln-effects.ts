export function useAlderfenEffect(effect: () => void | (() => void), _dependencies: unknown[]) {
  return effect();
}
export function useAlderfenCallback<T>(callback: T, _dependencies: unknown[]) {
  return callback;
}
