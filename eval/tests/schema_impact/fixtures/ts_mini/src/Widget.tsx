import { request } from '@/index';
import useKey from 'swr';
import { useSWR as useNamedKey } from 'swr';
const item = '/legacy/item';

export function Widget() {
  useEffect(() => {
    request.get(`/legacy/item?title=${title}`); // CALLBACK
  }, []);
  useKey<Type.Item>(enabled ? [item, { params }] : null, () => // HOOK
    request.get(item), // HOOK_INLINE_FETCHER
  );
  useNamedKey(item, request.post); // POST_FETCHER_HOOK
  useKey(item, () => request.instance.put(item)); // INLINE_PUT_HOOK
  useKey({ item }, request.get); // NON_STRING_KEY
  useKey([buildKey(item), params], request.get); // ARRAY_NON_STRING_KEY
  useKey(enabled ? '/legacy/red' : '/legacy/blue', request.get); // CONDITIONAL_URL
  useKey(item); // HOOK_WITHOUT_FETCHER_SPEC
  useKey(item, (url) => request.via(url, method)); // DYNAMIC_FETCHER_METHOD
  return <div />;
}
