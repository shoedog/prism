import quay from './quay';
import { useQuayKey as useSable } from './key';

const trail = '/cairns';
export const restoreCairn = (path: string) => quay.put(path); // PUT dynamic when called with an arbitrary path
export function cards(enabled: boolean, mark: number, props: object, arbitraryKey: unknown) {
  useSable([trail, props], quay.get); // GET /cairns
  useSable(trail, quay.instance.post); // POST /cairns
  useSable(trail, quay.post); // POST /cairns
  useSable(trail, // PUT /cairns
    async (url) => await quay.instance.put(url),
  );
  useSable(trail, // PUT /lanterns
    () => quay.instance.put('/lanterns'),
  );
  useSable(trail, restoreCairn); // PUT /cairns
  useSable(trail, (url) => ({ url }));
  useSable(trail); // GET /cairns
  useSable(enabled ? '/cairns' : null, quay.get); // GET /cairns
  useSable(enabled ? undefined : ['/cairns', props], quay.get); // GET /cairns
  useSable(enabled ? false : '/cairns', quay.get); // GET /cairns
  useSable(enabled ? '' : '/cairns', quay.get); // GET /cairns
  useSable([`/cairns/${mark}`, arbitraryKey], quay.get); // GET /cairns/{}
  useSable(['/cairns', '/ignored'], quay.get); // GET /cairns
  useSable({ trail }, quay.get); // GET with an object key
  useSable([arbitraryKey, '/cairns'], quay.get); // GET with an unknown first key element
  useSable([, '/cairns'], quay.get); // GET with an absent first key element
  useSable([], quay.get); // GET with an empty key array
  useSable(enabled ? '/basalt' : '/calcite', quay.get); // GET /basalt or GET /calcite
}
