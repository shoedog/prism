# Synthetic HTTP call shapes

Each block is an independent fixture. Module labels mark separate files. Bare local imports resolve from the fixture root, `@linden/kiln-slips` aliases `linden/kiln-slips`, and `linden/kiln-courier` uses the factory in s01. Supply stubs for the invented dependencies. Courier paths assume an empty deployment mount. Patterns omit queries, and `{}` denotes one whole segment.

**s01** — A factory exports a client built from an origin, an optional deployment mount, and a fixed prefix, and a bare local import calls it with a slash-prefixed literal. **URL** `GET /fernway/widgets`
```ts
// linden/kiln-courier
import { mintLindenCourier } from '@linden-kit/kiln-transport';
import { lindenOrigin, lindenMount } from 'linden/kiln-settings';
const lindenStem = lindenMount === '/' ? '' : lindenMount;
export const lindenCourier = mintLindenCourier.create({
  baseURL: `${lindenOrigin.replace(/\/$/, '')}${lindenStem}/fernway/`,
});
// linden/kiln-tally
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = () => lindenCourier.get('/widgets');
```
**s02** — A renamed named import calls the client with a literal that has no leading slash. **URL** `GET /fernway/widgets`
```ts
import { lindenCourier as birchCourier } from 'linden/kiln-courier';
export const tallyBirch = () => birchCourier.get('widgets');
```
**s03** — A path alias resolves to an exported arrow wrapper that returns the client promise. **URL** `GET /fernway/invoices`
```ts
// linden/kiln-slips
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = () => lindenCourier.get('invoices');
// linden/kiln-tally
import { tallyLinden } from '@linden/kiln-slips';
export const countLinden = () => tallyLinden();
```
**s04** — A template without substitutions supplies a fixed path in either slash spelling. **URL** `GET /fernway/parcels`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = () => lindenCourier.get(`/parcels`);
export const tallyBirch = () => lindenCourier.get(`parcels`);
```

**s05** — A substitution fills one complete segment. **URL** `GET /fernway/orchards/{}`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = (lindenMark: string) => lindenCourier.get(`/orchards/${lindenMark}`);
```
**s06** — Two substitutions fill separate complete segments in a template without a leading slash. **URL** `GET /fernway/orchards/{}/{}`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = (lindenMark: string, lindenStamp: string) =>
  lindenCourier.get(`orchards/${lindenMark}/${lindenStamp}`);
```
**s07** — A URI-encoding call supplies a whole segment, keeping slashes in the input encoded. **URL** `GET /fernway/bouquets/{}`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = (lindenMark: string) => lindenCourier.get(`/bouquets/${encodeURIComponent(lindenMark)}`);
```
**s08** — A literal query delimiter separates query interpolation from the path. **URL** `GET /fernway/baskets/{}`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = (lindenMark: string, lindenTint: string) =>
  lindenCourier.get(`/baskets/${lindenMark}?lindenTint=${lindenTint}`);
```

**s09** — A substitution shares its segment with a fixed suffix or a percent-encoded slash prefix. **URL** `dynamic` because the final segment mixes fixed text with a runtime value.
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = (lindenMark: string) => lindenCourier.get(`/crates/%2F${lindenMark}`);
export const tallyBirch = (lindenMark: string) => lindenCourier.get(`/crates/${lindenMark}-ridge-gold`);
```
**s10** — The URL is initialized once in a local constant and passed by identifier. **URL** `GET /fernway/ledgers/{}`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export function tallyLinden(lindenMark: string) {
  const lindenTrail = `/ledgers/${lindenMark}`;
  return lindenCourier.get(lindenTrail);
}
```
**s11** — A mutable URL starts with a fixed path and conditionally appends a serialized query. **URL** `GET /fernway/shelves`
```ts
import { lindenCourier } from 'linden/kiln-courier';
import { inkLindenQuery } from '@linden-kit/kiln-pen';
export function tallyLinden(lindenFacets: Record<string, string>) {
  let lindenTrail = '/shelves';
  if (Object.keys(lindenFacets).length) lindenTrail += `?${inkLindenQuery(lindenFacets)}`;
  return lindenCourier.get(lindenTrail);
}
```
**s12** — A template glues a prebuilt suffix to the path, with the query delimiter supplied by the suffix. **URL** `GET /fernway/pallets`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export function tallyLinden(lindenFacets: Record<string, string>) {
  let lindenTail = '';
  if (Object.keys(lindenFacets).length) lindenTail += `?${new URLSearchParams(lindenFacets)}`;
  return lindenCourier.get(`/pallets${lindenTail}`);
}
```

**s13** — An interpolated ternary suffix supplies either a slash and segment or an empty string. **URL** `GET /fernway/acorns` or `GET /fernway/acorns/{}`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export function tallyLinden(lindenMark?: string) {
  const lindenTail = lindenMark ? `/${lindenMark}` : '';
  return lindenCourier.get(`/acorns${lindenTail}`);
}
```
**s14** — A local ternary or inline nullish coalescing can empty a whole-segment substitution and leave the preceding slash in place. **URL** `GET /fernway/pebbles/` or `GET /fernway/pebbles/{}`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export function tallyLinden(lindenMark?: string) {
  const lindenLeaf = lindenMark ? `${lindenMark}` : '';
  return lindenCourier.get(`/pebbles/${lindenLeaf}`);
}
export const tallyBirch = (lindenMark?: string) => lindenCourier.get(`/pebbles/${lindenMark ?? ''}`);
```
**s15** — An interpolated short-circuit expression stringifies its falsy result. **URL** `dynamic` because a truthy result adds a segment while `undefined`, `null`, and `false` glue their text to the last segment.
```ts
import { lindenCourier } from 'linden/kiln-courier';
export function tallyLinden(lindenMark?: string | null | false) {
  const lindenTail = lindenMark && `/${lindenMark}`;
  return lindenCourier.get(`/booklets${lindenTail}`);
}
```
**s16** — A condition can extend a mutable URL by a path segment, with or without adding a query. **URL** `GET /fernway/brochures`, `GET /fernway/brochures/bunting`, or `GET /fernway/brochures/{}`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export function tallyLinden(lindenFacets?: Record<string, string>) {
  let lindenTrail = '/brochures';
  if (lindenFacets) lindenTrail += `/bunting?${new URLSearchParams(lindenFacets)}`;
  return lindenCourier.get(lindenTrail);
}
export function tallyBirch(lindenMark?: string) {
  let lindenTrail = '/brochures';
  if (lindenMark) lindenTrail = `${lindenTrail}/${lindenMark}`;
  return lindenCourier.get(lindenTrail);
}
```

**s17** — Both if/else branches assign a nonempty URL before a single verb call. **URL** `GET /fernway/bluebells/{}` or `GET /fernway/bluebells/{}/{}`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export function tallyLinden(lindenMark: string, lindenStamp?: string) {
  let lindenTrail: string;
  if (lindenStamp) lindenTrail = `/bluebells/${lindenStamp}/${lindenMark}`;
  else lindenTrail = `/bluebells/${lindenMark}`;
  return lindenCourier.get(lindenTrail);
}
```
**s18** — A ternary within one interpolation selects between two nonempty literal segments. **URL** `GET /fernway/booklets/baskets/{}` or `GET /fernway/booklets/crates/{}`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export function tallyLinden(lindenWide: boolean, lindenMark: string) {
  const lindenTrail = `/booklets/${lindenWide ? 'baskets' : 'crates'}/${lindenMark}`;
  return lindenCourier.get(lindenTrail);
}
```
**s19** — A fixed member of an imported constant object supplies a literal segment. **URL** `GET /fernway/brochures/chalkboard/{}`
```ts
// linden/kiln-ink
export const lindenSwatches = { lindenMatte: 'chalkboard' } as const;
// linden/kiln-tally
import { lindenSwatches } from 'linden/kiln-ink';
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = (lindenMark: string) =>
  lindenCourier.get(`/brochures/${lindenSwatches.lindenMatte}/${lindenMark}`);
```
**s20** — One interpolation carries an unencoded path tail. **URL** `dynamic` because slashes in the tail can add more than one segment.
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = (lindenTrail: string) => lindenCourier.get(`/greenhouses/${lindenTrail.replaceAll('\\', '/')}`);
```

**s21** — A runtime table key selects a record whose URL is read directly, assigned to a local, or destructured. **URL** `GET /fernway/crates` or `GET /fernway/pallets` according to the key.
```ts
import { lindenCourier } from 'linden/kiln-courier';
const lindenMap = { lindenAsh: { lindenTrail: '/crates' }, lindenElm: { lindenTrail: '/pallets' } };
type LindenKey = keyof typeof lindenMap;
export const tallyLinden = (lindenKey: LindenKey) => lindenCourier.get(lindenMap[lindenKey].lindenTrail);
export function tallyBirch(lindenKey: LindenKey) {
  const lindenTrail = lindenMap[lindenKey].lindenTrail;
  return lindenCourier.get(lindenTrail);
}
export function tallyAlder(lindenKey: LindenKey) {
  const { lindenTrail } = lindenMap[lindenKey];
  return lindenCourier.get(lindenTrail);
}
```
**s22** — A condition picks a verb, binds the receiver and URL, and later invokes the bound function with a body. **URL** `dynamic` because the condition switches between POST and PUT and a runtime record supplies the path.
```ts
import { lindenCourier } from 'linden/kiln-courier';
export function stampLinden(lindenNew: boolean, lindenCard: { lindenFresh: string; lindenExisting: string }) {
  const lindenSend = lindenNew
    ? lindenCourier.post.bind(lindenCourier, lindenCard.lindenFresh)
    : lindenCourier.put.bind(lindenCourier, lindenCard.lindenExisting);
  return lindenSend({ lindenCount: 2 });
}
```
**s23** — A multiline template carries a raw trailing newline after a substitution. **URL** `GET /fernway/invoices/{}` after URL parsing removes the newline.
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = (lindenMark: string) => lindenCourier.get(
  `/invoices/${lindenMark}
`,
);
```
**s24** — The URL repeats the client's prefix as an interior path segment. **URL** `GET /fernway/lozenges/fernway/{}`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = (lindenMark: string) => lindenCourier.get(`/lozenges/fernway/${lindenMark}`);
```

**s25** — A same-verb literal path overlaps a parameterized pattern. **URL** `GET /fernway/orchards/bunting` and `GET /fernway/orchards/{}` with the literal match taking precedence.
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = () => lindenCourier.get('/orchards/bunting');
export const tallyBirch = (lindenMark: string) => lindenCourier.get(`/orchards/${lindenMark}`);
```
**s26** — A nested TypeScript type argument appears between the verb member and the call parentheses. **URL** `PUT /fernway/shelves`
```ts
import { lindenCourier } from 'linden/kiln-courier';
type LindenSheet<LindenCell> = { lindenCells: LindenCell[] };
export const stampLinden = () =>
  lindenCourier.put<LindenSheet<LindenSheet<number>>>('/shelves', { lindenCount: 2 });
```
**s27** — GET takes a second argument for query parameters, cancellation, binary mode, and download progress. **URL** `GET /fernway/booklets`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const tallyLinden = (lindenAbort: AbortSignal, lindenTick: () => void) =>
  lindenCourier.get('/booklets', {
    params: { lindenTint: 'bunting' }, signal: lindenAbort,
    responseType: 'blob', onDownloadProgress: lindenTick,
  });
```
**s28** — DELETE places its body inside the second argument's options object. **URL** `DELETE /fernway/crates`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const clearLinden = () => lindenCourier.delete('/crates', { data: { lindenCount: 2 } });
```

**s29** — POST and PUT take a positional body that can itself be an envelope, with options in the third argument. **URL** `PUT /fernway/pallets` and `POST /fernway/pallets`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export const stampLinden = (lindenAbort: AbortSignal) =>
  lindenCourier.put('/pallets', { lindenCount: 2 }, { signal: lindenAbort });
export const stampBirch = () => lindenCourier.post('/pallets', { lindenPacket: { lindenCount: 2 } });
```
**s30** — Shared interceptors add a request header and pass through responses while propagating rejections. **URL** `GET /fernway/ledgers`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export function primeLinden(lindenSeal: string) {
  lindenCourier.interceptors.request.use(lindenDraft => {
    lindenDraft.headers['X-Linden-Seal'] = lindenSeal;
    return lindenDraft;
  });
  lindenCourier.interceptors.response.use(lindenReply => lindenReply, lindenFault => Promise.reject(lindenFault));
}
export const tallyLinden = () => lindenCourier.get('/ledgers');
```
**s31** — A hook registers a response interceptor during an effect and ejects it during cleanup. **URL** `GET /fernway/bouquets`
```ts
import { useLindenEffect } from '@linden-kit/kiln-effects';
import { lindenCourier } from 'linden/kiln-courier';
export function useLindenWatch(lindenNotify: () => void) {
  useLindenEffect(() => {
    const lindenTicket = lindenCourier.interceptors.response.use(lindenReply => lindenReply, lindenFault => {
      if (lindenFault.response?.status === 401) lindenNotify();
      return Promise.reject(lindenFault);
    });
    return () => lindenCourier.interceptors.response.eject(lindenTicket);
  }, [lindenNotify]);
}
export const tallyLinden = () => lindenCourier.get('/bouquets');
```
**s32** — A named function declaration wraps the verb, and its caller awaits it through a bare local import. **URL** `GET /fernway/bluebells`
```ts
// linden/kiln-slips
import { lindenCourier } from 'linden/kiln-courier';
export function tallyLinden() { return lindenCourier.get('/bluebells'); }
// linden/kiln-tally
import { tallyLinden } from 'linden/kiln-slips';
export const countLinden = async () => (await tallyLinden()).data;
```

**s33** — An async wrapper awaits the verb and returns the response body directly. **URL** `GET /fernway/acorns`
```ts
import { lindenCourier } from 'linden/kiln-courier';
export async function tallyLinden() {
  const lindenReply = await lindenCourier.get('/acorns');
  return lindenReply.data;
}
```
**s34** — A runtime condition selects between two wrapper functions with different fixed verbs. **URL** `POST /fernway/bouquets` or `PUT /fernway/bouquets`
```ts
import { lindenCourier } from 'linden/kiln-courier';
const stampLinden = (lindenSlip: object) => lindenCourier.post('/bouquets', lindenSlip);
const stampBirch = (lindenSlip: object) => lindenCourier.put('/bouquets', lindenSlip);
export function fileLinden(lindenNew: boolean, lindenSlip: object) {
  const lindenSubmit = lindenNew ? stampLinden : stampBirch;
  return lindenSubmit(lindenSlip);
}
export function fileBirch(lindenNew: boolean, lindenSlip: object) {
  let lindenSubmit;
  if (lindenNew) lindenSubmit = stampLinden;
  else lindenSubmit = stampBirch;
  return lindenSubmit(lindenSlip);
}
```
**s35** — A component receives a wrapper as a callback prop and invokes it on a UI event. **URL** `GET /fernway/greenhouses`
```tsx
import { lindenCourier } from 'linden/kiln-courier';
const tallyLinden = () => lindenCourier.get('/greenhouses');
function LindenTile({ lindenTap }: { lindenTap: () => unknown }) {
  return <button onClick={() => { void lindenTap(); }} />;
}
export const LindenBoard = () => <LindenTile lindenTap={tallyLinden} />;
```
**s36** — A custom hook takes a wrapper and invokes it inside a memoized callback. **URL** `PUT /fernway/lozenges/{}`
```ts
import { useLindenCallback } from '@linden-kit/kiln-effects';
import { lindenCourier } from 'linden/kiln-courier';
const stampLinden = (lindenMark: string, lindenSlip: object) =>
  lindenCourier.put(`/lozenges/${lindenMark}`, lindenSlip);
function useLindenStamp(lindenSend: typeof stampLinden) {
  return useLindenCallback((lindenMark: string, lindenSlip: object) =>
    lindenSend(lindenMark, lindenSlip), [lindenSend]);
}
export const useLindenBoard = () => useLindenStamp(stampLinden);
```

**s37** — A caller starts several requests through wrappers and joins their promises in a batch. **URL** `GET /fernway/invoices` and `GET /fernway/brochures`
```ts
import { lindenCourier } from 'linden/kiln-courier';
const tallyLinden = () => lindenCourier.get('/invoices');
const tallyBirch = () => lindenCourier.get('/brochures');
export const countLinden = () => Promise.allSettled([tallyLinden(), tallyBirch()]);
export const countBirch = () => Promise.all([tallyLinden(), tallyBirch()]);
```
**s38** — Built-in fetch defaults to GET and receives the URL from a helper that normalizes a deployment mount. **URL** `dynamic` because the helper places `/receipt-seal.json` at the root or under the configured mount.
```ts
import { lindenMount } from 'linden/kiln-settings';
function findLinden(lindenStem = lindenMount) {
  const lindenClean = lindenStem.split('/').filter(Boolean).join('/');
  return lindenClean ? `/${lindenClean}/receipt-seal.json` : '/receipt-seal.json';
}
export const tallyLinden = (lindenAbort: AbortSignal) => fetch(findLinden(), { cache: 'no-store', signal: lindenAbort });
```
**s39** — A wrapper probes a static script by passing a caller-built identifier into built-in fetch. **URL** `dynamic` because GET targets `/sprig-loader.js` under the caller's deployment mount.
```ts
import { lindenPublicStem } from 'linden/kiln-settings';
const probeLinden = (lindenTrail: string) => fetch(lindenTrail);
export const tallyLinden = () => probeLinden(`${lindenPublicStem}/sprig-loader.js`);
```
**s40** — A browser download link receives a templated URL through its anchor attribute. **URL** `GET /fernway/brochures/{}` on link activation at the root mount.
```tsx
import { lindenOrigin, lindenMount } from 'linden/kiln-settings';
const lindenStem = lindenMount === '/' ? '' : lindenMount;
export const LindenLink = ({ lindenMark }: { lindenMark: string }) =>
  <a href={`${lindenOrigin}${lindenStem}/fernway/brochures/${lindenMark}`} download>Brochure</a>;
```

**s41** — A conditional path prefix goes into a Markdown image URL that the renderer loads. **URL** `GET /fernway/bouquets/baskets/{}/{}` or `GET /fernway/bouquets/crates/{}` on rendering.
```ts
import { lindenOrigin } from 'linden/kiln-settings';
export function sketchLinden(lindenWide: boolean, lindenMark: string, lindenStamp: string) {
  const lindenTrail = lindenWide ? `bouquets/baskets/${lindenStamp}` : 'bouquets/crates';
  return `![](${lindenOrigin}/fernway/${lindenTrail}/${lindenMark})`;
}
```
**s42** — A browser link concatenates an origin prefix and a helper-joined path. **URL** `GET /booklets/{}` on link activation with a slash-terminated origin and single-segment key.
```tsx
import { joinLindenTrail } from '@linden-kit/kiln-trail';
export const LindenLink = ({ lindenRoot, lindenMark }: { lindenRoot: string; lindenMark: string }) =>
  <a href={lindenRoot + joinLindenTrail('booklets', lindenMark)}>Booklet</a>;
```
**s43** — An upload component receives a templated destination prop and chooses the HTTP operations itself. **URL** `POST /fernway/{}/{}/{}` for upload, with follow-up methods and URLs chosen by the component.
```tsx
import { LindenUploader } from '@linden-kit/kiln-upload';
import { lindenOrigin } from 'linden/kiln-settings';
export const LindenShelf = (lindenPick: { lindenBin: string; lindenMark: string; lindenStamp: string }) =>
  <LindenUploader lindenServer={`${lindenOrigin}/fernway/${lindenPick.lindenBin}/${lindenPick.lindenMark}/${lindenPick.lindenStamp}`} />;
```
**s44** — An SDK accepts a configured base and derives HTTP calls from startup and identity operations. **URL** `dynamic` because the SDK supplies the methods and the paths beneath the configured base.
```ts
import { lindenRoster } from '@linden-kit/kiln-roster';
import { lindenRelay, lindenBadge } from 'linden/kiln-settings';
export const openLinden = () => lindenRoster.beginLinden({ lindenBase: lindenRelay, lindenBadge });
export const markLinden = (lindenMark: string) => lindenRoster.attachLinden(lindenMark);
```

**s45** — An HTTP exporter receives its destination through constructor options and issues POSTs internally. **URL** `dynamic` because POST uses the runtime exporter destination.
```ts
import { LindenExporter } from '@linden-kit/kiln-pulse';
import { lindenOutlet } from 'linden/kiln-settings';
export const lindenExporter = new LindenExporter({ lindenDestination: lindenOutlet });
```
