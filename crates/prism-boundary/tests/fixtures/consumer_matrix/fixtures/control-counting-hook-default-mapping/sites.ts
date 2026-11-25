import quay from './quay';
import { useQuayKey } from './key';

const key = '/cairns';
declare const later: (callback: () => unknown) => void;
export function count(method: string) {
  quay.upload(key, method); // dynamic method /cairns
  quay.cellar.invoke(key, method); // dynamic method /cairns
  quay['instance.put'](key, method); // dynamic method /cairns
  useQuayKey('/cairns');
    quay.get,
  );
}

export function callback() {
  later(function () {
    return quay.get('/calipers'); // GET /calipers
  });
}

export function inline() {
  return useQuayKey('/cairns', // GET /cairns
    (url) => quay.get(url),
  );
}
